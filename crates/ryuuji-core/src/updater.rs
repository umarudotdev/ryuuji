use std::path::PathBuf;

use serde::Deserialize;

use crate::config::AppConfig;
use crate::error::RyuujiError;

const GITHUB_REPO: &str = "umarudotdev/ryuuji";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Types ───────────────────────────────────────────────────────────

/// How the app was installed — determines whether self-update is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKind {
    AppImage,
    DebPackage,
    WindowsPortable,
    WindowsInstaller,
    Unknown,
}

impl InstallKind {
    /// Whether this install kind supports in-place self-update.
    pub fn supports_self_update(self) -> bool {
        matches!(
            self,
            Self::AppImage | Self::WindowsPortable | Self::WindowsInstaller
        )
    }
}

/// Information about an available update.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: semver::Version,
    pub tag_name: String,
    pub release_url: String,
    /// `None` for notify-only install kinds (deb).
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub is_prerelease: bool,
    pub body: String,
}

/// Session-scoped update state machine.
#[derive(Debug, Clone, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    Available(Box<UpdateInfo>),
    Downloading(Box<UpdateInfo>),
    ReadyToApply {
        info: Box<UpdateInfo>,
        path: PathBuf,
    },
    ReadyToRestart,
    Failed(String),
    /// Update available but self-update not supported — user must download manually.
    NotifyOnly(Box<UpdateInfo>),
}

// ── GitHub API types (private) ──────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    prerelease: bool,
    #[serde(default)]
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

// ── Install kind detection ──────────────────────────────────────────

/// Detect how the app was installed based on the current executable path
/// and environment variables.
pub fn detect_install_kind() -> InstallKind {
    #[cfg(target_os = "linux")]
    {
        if std::env::var("APPIMAGE").is_ok() {
            return InstallKind::AppImage;
        }
        if let Ok(exe) = std::env::current_exe() {
            let path = exe.display().to_string();
            if path.starts_with("/usr/bin") || path.starts_with("/usr/local/bin") {
                return InstallKind::DebPackage;
            }
        }
        InstallKind::Unknown
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(exe) = std::env::current_exe() {
            let path = exe.display().to_string();
            let program_files =
                std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
            let program_files_x86 = std::env::var("ProgramFiles(x86)")
                .unwrap_or_else(|_| r"C:\Program Files (x86)".into());
            if path.starts_with(&program_files) || path.starts_with(&program_files_x86) {
                return InstallKind::WindowsInstaller;
            }
            return InstallKind::WindowsPortable;
        }
        InstallKind::Unknown
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        InstallKind::Unknown
    }
}

// ── Artifact name mapping ───────────────────────────────────────────

/// Return the expected release artifact filename for a given install kind and version.
fn asset_name_for(kind: InstallKind, version: &semver::Version) -> Option<String> {
    match kind {
        InstallKind::AppImage => Some(format!("Ryuuji-{version}-x86_64.AppImage")),
        InstallKind::WindowsPortable => Some(format!("ryuuji-{version}-windows-x64-portable.zip")),
        InstallKind::WindowsInstaller => Some(format!("ryuuji-{version}-windows-x64-setup.exe")),
        // Package-managed installs (deb) don't self-update.
        InstallKind::DebPackage | InstallKind::Unknown => None,
    }
}

// ── Core update logic ───────────────────────────────────────────────

/// Parse a version string from a git tag (strips leading 'v' if present).
fn parse_tag_version(tag: &str) -> Option<semver::Version> {
    let raw = tag.strip_prefix('v').unwrap_or(tag);
    semver::Version::parse(raw).ok()
}

/// Find the best release from a list of GitHub releases.
fn find_best_release(
    releases: &[GitHubRelease],
    include_prerelease: bool,
    install_kind: InstallKind,
) -> Option<UpdateInfo> {
    let current = semver::Version::parse(CURRENT_VERSION).ok()?;

    releases
        .iter()
        .filter(|r| include_prerelease || !r.prerelease)
        .filter_map(|r| {
            let version = parse_tag_version(&r.tag_name)?;
            if version <= current {
                return None;
            }
            let expected_asset = asset_name_for(install_kind, &version);
            let (download_url, asset_name) = if let Some(ref expected) = expected_asset {
                let asset = r.assets.iter().find(|a| &a.name == expected)?;
                (
                    Some(asset.browser_download_url.clone()),
                    Some(asset.name.clone()),
                )
            } else {
                (None, None)
            };

            Some(UpdateInfo {
                version,
                tag_name: r.tag_name.clone(),
                release_url: r.html_url.clone(),
                download_url,
                asset_name,
                is_prerelease: r.prerelease,
                body: r.body.clone().unwrap_or_default(),
            })
        })
        .max_by(|a, b| a.version.cmp(&b.version))
}

/// Check GitHub Releases for a newer version.
pub async fn check_for_update(include_prerelease: bool) -> Result<Option<UpdateInfo>, RyuujiError> {
    let install_kind = detect_install_kind();
    let client = reqwest::Client::builder()
        .user_agent(format!("ryuuji/{CURRENT_VERSION}"))
        .build()
        .map_err(|e| RyuujiError::Update(e.to_string()))?;

    let releases: Vec<GitHubRelease> = if include_prerelease {
        let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases?per_page=10");
        client
            .get(&url)
            .send()
            .await
            .map_err(|e| RyuujiError::Update(e.to_string()))?
            .json()
            .await
            .map_err(|e| RyuujiError::Update(e.to_string()))?
    } else {
        let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| RyuujiError::Update(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let release: GitHubRelease = resp
            .json()
            .await
            .map_err(|e| RyuujiError::Update(e.to_string()))?;
        vec![release]
    };

    let info = find_best_release(&releases, include_prerelease, install_kind);

    // For install kinds that don't support self-update but still have a newer version,
    // return UpdateInfo with download_url = None (the GUI will show notify-only UI).
    if let Some(ref info) = info {
        if !install_kind.supports_self_update() {
            return Ok(Some(UpdateInfo {
                download_url: None,
                asset_name: None,
                ..info.clone()
            }));
        }
    }

    Ok(info)
}

/// Download the update artifact to a temp directory under the app data dir.
pub async fn download_update(info: &UpdateInfo) -> Result<PathBuf, RyuujiError> {
    let download_url = info
        .download_url
        .as_ref()
        .ok_or_else(|| RyuujiError::Update("no download URL for this install type".into()))?;

    let asset_name = info
        .asset_name
        .as_ref()
        .ok_or_else(|| RyuujiError::Update("no asset name".into()))?;

    let update_dir = AppConfig::project_dirs()
        .map(|d| d.data_dir().join("update"))
        .unwrap_or_else(|| PathBuf::from("update"));

    std::fs::create_dir_all(&update_dir)?;

    let dest = update_dir.join(asset_name);

    let client = reqwest::Client::builder()
        .user_agent(format!("ryuuji/{CURRENT_VERSION}"))
        .build()
        .map_err(|e| RyuujiError::Update(e.to_string()))?;

    let bytes = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| RyuujiError::Update(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| RyuujiError::Update(e.to_string()))?;

    std::fs::write(&dest, &bytes)?;

    Ok(dest)
}

/// Apply the downloaded update in place. Platform-specific.
pub fn apply_update(artifact_path: &std::path::Path) -> Result<(), RyuujiError> {
    let install_kind = detect_install_kind();
    match install_kind {
        InstallKind::AppImage => apply_appimage(artifact_path),
        InstallKind::WindowsPortable => apply_windows_portable(artifact_path),
        InstallKind::WindowsInstaller => apply_windows_installer(artifact_path),
        _ => Err(RyuujiError::Update(
            "self-update not supported for this install type".into(),
        )),
    }
}

#[cfg(target_os = "linux")]
fn apply_appimage(artifact_path: &std::path::Path) -> Result<(), RyuujiError> {
    use std::os::unix::fs::PermissionsExt;

    let appimage_path = std::env::var("APPIMAGE")
        .map(PathBuf::from)
        .map_err(|_| RyuujiError::Update("APPIMAGE env var not set".into()))?;

    let old_path = appimage_path.with_extension("old");

    // Rename current → .old
    std::fs::rename(&appimage_path, &old_path)?;

    // Copy new artifact in place
    if let Err(e) = std::fs::copy(artifact_path, &appimage_path) {
        // Rollback: restore old binary
        let _ = std::fs::rename(&old_path, &appimage_path);
        return Err(RyuujiError::Update(format!(
            "failed to copy new binary: {e}"
        )));
    }

    // Make executable
    let mut perms = std::fs::metadata(&appimage_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&appimage_path, perms)?;

    // Clean up old binary and downloaded artifact
    let _ = std::fs::remove_file(&old_path);
    let _ = std::fs::remove_file(artifact_path);

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn apply_appimage(_artifact_path: &std::path::Path) -> Result<(), RyuujiError> {
    Err(RyuujiError::Update(
        "AppImage updates only supported on Linux".into(),
    ))
}

#[cfg(target_os = "windows")]
fn apply_windows_portable(artifact_path: &std::path::Path) -> Result<(), RyuujiError> {
    let current_exe = std::env::current_exe().map_err(|e| RyuujiError::Update(e.to_string()))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| RyuujiError::Update("cannot determine exe directory".into()))?;

    // Pre-cleanup: remove leftover .old files from previous updates
    cleanup_old_executables(exe_dir);

    // Clean leftover staging dir from a previous failed attempt
    let extract_dir = exe_dir.join("_update_staging");
    let _ = std::fs::remove_dir_all(&extract_dir);

    // Extract zip via PowerShell (use -LiteralPath for paths with special chars)
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
                artifact_path.display(),
                extract_dir.display()
            ),
        ])
        .status()
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&extract_dir);
            RyuujiError::Update(format!("failed to run PowerShell: {e}"))
        })?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&extract_dir);
        return Err(RyuujiError::Update("Expand-Archive failed".into()));
    }

    // Find the new exe in the extracted directory
    let new_exe = find_exe_in_dir(&extract_dir)?;

    // Rename current exe → .old.exe
    let old_exe = current_exe.with_extension("old.exe");
    // On Windows, rename doesn't overwrite — ensure target is removed first
    if old_exe.exists() {
        if let Err(e) = std::fs::remove_file(&old_exe) {
            // If we can't remove .old.exe (e.g. locked by AV), use a timestamped name
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let fallback = exe_dir.join(format!("ryuuji.old.{ts}.exe"));
            std::fs::rename(&current_exe, &fallback).map_err(|e2| {
                let _ = std::fs::remove_dir_all(&extract_dir);
                RyuujiError::Update(format!(
                    "failed to rename current exe (old.exe locked: {e}): {e2}"
                ))
            })?;
            return finish_portable_apply(&new_exe, &current_exe, &extract_dir, artifact_path);
        }
    }
    std::fs::rename(&current_exe, &old_exe).map_err(|e| {
        let _ = std::fs::remove_dir_all(&extract_dir);
        RyuujiError::Update(format!("failed to rename current exe: {e}"))
    })?;

    finish_portable_apply(&new_exe, &current_exe, &extract_dir, artifact_path)
}

/// Copy new exe into place and clean up staging artifacts.
#[cfg(target_os = "windows")]
fn finish_portable_apply(
    new_exe: &std::path::Path,
    target: &std::path::Path,
    extract_dir: &std::path::Path,
    artifact_path: &std::path::Path,
) -> Result<(), RyuujiError> {
    if let Err(e) = std::fs::copy(new_exe, target) {
        // Rollback: try to restore from .old.exe
        let old_exe = target.with_extension("old.exe");
        let _ = std::fs::rename(&old_exe, target);
        let _ = std::fs::remove_dir_all(extract_dir);
        return Err(RyuujiError::Update(format!("failed to copy new exe: {e}")));
    }

    let _ = std::fs::remove_dir_all(extract_dir);
    let _ = std::fs::remove_file(artifact_path);
    Ok(())
}

#[cfg(target_os = "windows")]
fn find_exe_in_dir(dir: &std::path::Path) -> Result<PathBuf, RyuujiError> {
    for entry in std::fs::read_dir(dir).map_err(|e| RyuujiError::Update(e.to_string()))? {
        let entry = entry.map_err(|e| RyuujiError::Update(e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "exe") {
            return Ok(path);
        }
        if path.is_dir() {
            if let Ok(found) = find_exe_in_dir(&path) {
                return Ok(found);
            }
        }
    }
    Err(RyuujiError::Update(
        "no .exe found in extracted archive".into(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn apply_windows_portable(_artifact_path: &std::path::Path) -> Result<(), RyuujiError> {
    Err(RyuujiError::Update(
        "Windows portable updates only supported on Windows".into(),
    ))
}

/// Apply update via NSIS installer — run the downloaded setup.exe with elevation.
/// The installer handles uninstalling the old version and installing the new one.
/// Uses `ShellExecuteW` with the `runas` verb to request UAC elevation, since
/// the installer needs admin rights to write to Program Files.
/// After this returns, the caller should exit (the installer replaces files in place).
#[cfg(target_os = "windows")]
fn apply_windows_installer(artifact_path: &std::path::Path) -> Result<(), RyuujiError> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut std::ffi::c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show_cmd: i32,
        ) -> *mut std::ffi::c_void;
    }

    fn to_wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn path_to_wide(p: &std::path::Path) -> Vec<u16> {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let verb = to_wide("runas");
    let file = path_to_wide(artifact_path);
    let params = to_wide("/S"); // NSIS silent install

    // SAFETY: ShellExecuteW is a well-defined Windows API. All string pointers are
    // valid null-terminated wide strings. NULL pointers are permitted for hwnd and directory.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        )
    };

    // ShellExecuteW returns a value > 32 on success.
    if (result as usize) <= 32 {
        return Err(RyuujiError::Update(format!(
            "failed to launch installer with elevation (ShellExecute returned {})",
            result as usize
        )));
    }

    // Clean up the downloaded installer
    let _ = std::fs::remove_file(artifact_path);

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn apply_windows_installer(_artifact_path: &std::path::Path) -> Result<(), RyuujiError> {
    Err(RyuujiError::Update(
        "Windows installer updates only supported on Windows".into(),
    ))
}

/// Spawn a new process from the current exe and exit.
pub fn restart() -> ! {
    #[cfg(target_os = "linux")]
    let exe = std::env::var("APPIMAGE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_exe().expect("cannot determine current exe"));

    #[cfg(not(target_os = "linux"))]
    let exe = {
        // After a portable update, current_exe() may resolve to the renamed .old.exe.
        // Use the exe directory + known binary name instead.
        let raw = std::env::current_exe().expect("cannot determine current exe");
        let dir = raw.parent().expect("exe has no parent dir");
        let canonical = dir.join("ryuuji.exe");
        if canonical.exists() {
            canonical
        } else {
            raw
        }
    };

    let _ = std::process::Command::new(&exe).spawn();
    std::process::exit(0);
}

/// Clean up leftover `.old` binaries from a previous update (Windows).
pub fn cleanup_old_binary() {
    #[cfg(target_os = "windows")]
    {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                cleanup_old_executables(dir);
            }
        }
    }
}

/// Remove old executables and staging artifacts left behind by previous updates.
#[cfg(target_os = "windows")]
fn cleanup_old_executables(dir: &std::path::Path) {
    let old = dir.join("ryuuji.old.exe");
    if old.exists() {
        if let Err(e) = std::fs::remove_file(&old) {
            tracing::warn!("failed to clean up {}: {e}", old.display());
        }
    }
    // Clean any timestamped ryuuji.old.*.exe files
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("ryuuji.old.") && name.ends_with(".exe") {
                if let Err(e) = std::fs::remove_file(entry.path()) {
                    tracing::warn!("failed to clean up {}: {e}", entry.path().display());
                }
            }
        }
    }
    // Clean leftover staging directory
    let staging = dir.join("_update_staging");
    if staging.exists() {
        let _ = std::fs::remove_dir_all(&staging);
    }
}

/// Return the current app version string.
pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_install_kind() {
        // In dev/test environment, should be Unknown (not running as AppImage or from /usr/bin)
        let kind = detect_install_kind();
        assert!(
            kind == InstallKind::Unknown || kind == InstallKind::WindowsPortable,
            "dev environment should be Unknown or WindowsPortable, got {kind:?}"
        );
    }

    #[test]
    fn test_asset_name_for_appimage() {
        let v = semver::Version::new(1, 2, 3);
        assert_eq!(
            asset_name_for(InstallKind::AppImage, &v),
            Some("Ryuuji-1.2.3-x86_64.AppImage".into())
        );
    }

    #[test]
    fn test_asset_name_for_windows_portable() {
        let v = semver::Version::new(0, 5, 0);
        assert_eq!(
            asset_name_for(InstallKind::WindowsPortable, &v),
            Some("ryuuji-0.5.0-windows-x64-portable.zip".into())
        );
    }

    #[test]
    fn test_asset_name_for_deb_is_none() {
        let v = semver::Version::new(1, 0, 0);
        assert_eq!(asset_name_for(InstallKind::DebPackage, &v), None);
    }

    #[test]
    fn test_asset_name_for_windows_installer() {
        let v = semver::Version::new(1, 0, 0);
        assert_eq!(
            asset_name_for(InstallKind::WindowsInstaller, &v),
            Some("ryuuji-1.0.0-windows-x64-setup.exe".into())
        );
    }

    #[test]
    fn test_parse_tag_version() {
        assert_eq!(
            parse_tag_version("v1.2.3"),
            Some(semver::Version::new(1, 2, 3))
        );
        assert_eq!(
            parse_tag_version("1.2.3"),
            Some(semver::Version::new(1, 2, 3))
        );
        assert_eq!(parse_tag_version("not-a-version"), None);
    }

    #[test]
    fn test_find_best_release_newer() {
        let releases = vec![GitHubRelease {
            tag_name: "v99.0.0".into(),
            html_url: "https://github.com/umarudotdev/ryuuji/releases/tag/v99.0.0".into(),
            prerelease: false,
            body: Some("Release notes".into()),
            assets: vec![GitHubAsset {
                name: "Ryuuji-99.0.0-x86_64.AppImage".into(),
                browser_download_url: "https://example.com/download".into(),
            }],
        }];

        let info = find_best_release(&releases, false, InstallKind::AppImage);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.version, semver::Version::new(99, 0, 0));
        assert_eq!(
            info.download_url,
            Some("https://example.com/download".into())
        );
    }

    #[test]
    fn test_find_best_release_up_to_date() {
        let current = semver::Version::parse(CURRENT_VERSION).unwrap();
        let releases = vec![GitHubRelease {
            tag_name: format!("v{current}"),
            html_url: "https://example.com".into(),
            prerelease: false,
            body: None,
            assets: vec![],
        }];

        let info = find_best_release(&releases, false, InstallKind::AppImage);
        assert!(info.is_none());
    }

    #[test]
    fn test_find_best_release_skips_bad_tags() {
        let releases = vec![
            GitHubRelease {
                tag_name: "nightly-2024-01-01".into(),
                html_url: "https://example.com".into(),
                prerelease: true,
                body: None,
                assets: vec![],
            },
            GitHubRelease {
                tag_name: "v99.0.0".into(),
                html_url: "https://example.com".into(),
                prerelease: false,
                body: None,
                assets: vec![GitHubAsset {
                    name: "Ryuuji-99.0.0-x86_64.AppImage".into(),
                    browser_download_url: "https://example.com/download".into(),
                }],
            },
        ];

        let info = find_best_release(&releases, false, InstallKind::AppImage);
        assert!(info.is_some());
        assert_eq!(info.unwrap().version, semver::Version::new(99, 0, 0));
    }

    #[test]
    fn test_github_release_json_parse() {
        let json = r###"{
            "tag_name": "v1.0.0",
            "html_url": "https://github.com/umarudotdev/ryuuji/releases/tag/v1.0.0",
            "prerelease": false,
            "body": "## What's new\n- Feature A\n- Bug fix B",
            "assets": [
                {
                    "name": "Ryuuji-1.0.0-x86_64.AppImage",
                    "browser_download_url": "https://github.com/umarudotdev/ryuuji/releases/download/v1.0.0/Ryuuji-1.0.0-x86_64.AppImage"
                },
                {
                    "name": "ryuuji-1.0.0-windows-x64-portable.zip",
                    "browser_download_url": "https://github.com/umarudotdev/ryuuji/releases/download/v1.0.0/ryuuji-1.0.0-windows-x64-portable.zip"
                }
            ]
        }"###;

        let release: GitHubRelease = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
        assert!(!release.prerelease);
        assert_eq!(release.assets.len(), 2);
        assert_eq!(release.assets[0].name, "Ryuuji-1.0.0-x86_64.AppImage");
    }

    #[test]
    fn test_notify_only_for_unsupported_install() {
        let releases = vec![GitHubRelease {
            tag_name: "v99.0.0".into(),
            html_url: "https://example.com/release".into(),
            prerelease: false,
            body: Some("notes".into()),
            assets: vec![],
        }];

        // DebPackage has no matching asset, so find_best_release returns None
        // (no asset matching the expected name).
        let info = find_best_release(&releases, false, InstallKind::DebPackage);
        // For deb, asset_name_for returns None, so find_best_release sets
        // download_url = None and asset_name = None, but still returns the release.
        assert!(info.is_some());
        let info = info.unwrap();
        assert!(info.download_url.is_none());
    }
}
