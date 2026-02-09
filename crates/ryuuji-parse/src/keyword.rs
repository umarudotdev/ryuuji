use bitflags::bitflags;
use phf::phf_map;

bitflags! {
    /// Flags controlling when and how a keyword matches.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KeywordFlags: u8 {
        /// Only match this keyword when it appears inside brackets.
        /// Prevents false positives for short/common words (e.g., "BD", "SD", "SP").
        const AMBIGUOUS = 0b0000_0001;
        /// This keyword must be followed by a number to match (e.g., "EP", "S", "Vol").
        const PREFIX_NUMBER = 0b0000_0010;
    }
}

/// The category a keyword belongs to, determining which element it populates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordKind {
    VideoCodec,
    AudioCodec,
    Resolution,
    Source,
    VideoTerm,
    AudioTerm,
    Language,
    Subtitles,
    ReleaseInfo,
    DeviceCompat,
    FileExtension,
    Episode,
    EpisodeType,
    Season,
    Part,
    Volume,
    ReleaseVersion,
    VideoColorDepth,
    VideoDynamicRange,
    VideoFrameRate,
    AudioChannels,
    StreamingSource,
}

/// A keyword entry with its kind and matching flags.
#[derive(Debug, Clone, Copy)]
pub struct KeywordEntry {
    pub kind: KeywordKind,
    pub flags: KeywordFlags,
}

impl KeywordEntry {
    const fn new(kind: KeywordKind) -> Self {
        Self {
            kind,
            flags: KeywordFlags::empty(),
        }
    }

    const fn ambiguous(kind: KeywordKind) -> Self {
        Self {
            kind,
            flags: KeywordFlags::AMBIGUOUS,
        }
    }

    const fn prefix(kind: KeywordKind) -> Self {
        Self {
            kind,
            flags: KeywordFlags::PREFIX_NUMBER,
        }
    }
}

/// Compile-time keyword lookup table.
/// All keys are UPPERCASE for case-insensitive matching.
pub static KEYWORDS: phf::Map<&'static str, KeywordEntry> = phf_map! {
    // ── Video codecs ─────────────────────────────────────────────
    "H264" => KeywordEntry::new(KeywordKind::VideoCodec),
    "H.264" => KeywordEntry::new(KeywordKind::VideoCodec),
    "X264" => KeywordEntry::new(KeywordKind::VideoCodec),
    "H265" => KeywordEntry::new(KeywordKind::VideoCodec),
    "H.265" => KeywordEntry::new(KeywordKind::VideoCodec),
    "X265" => KeywordEntry::new(KeywordKind::VideoCodec),
    "HEVC" => KeywordEntry::new(KeywordKind::VideoCodec),
    "AVC" => KeywordEntry::new(KeywordKind::VideoCodec),
    "AV1" => KeywordEntry::new(KeywordKind::VideoCodec),
    "XVID" => KeywordEntry::new(KeywordKind::VideoCodec),
    "DIVX" => KeywordEntry::new(KeywordKind::VideoCodec),
    "VP9" => KeywordEntry::new(KeywordKind::VideoCodec),
    "VP8" => KeywordEntry::new(KeywordKind::VideoCodec),
    "MPEG2" => KeywordEntry::new(KeywordKind::VideoCodec),
    "MPEG4" => KeywordEntry::new(KeywordKind::VideoCodec),
    "WMV3" => KeywordEntry::new(KeywordKind::VideoCodec),
    "VC-1" => KeywordEntry::new(KeywordKind::VideoCodec),
    "VC1" => KeywordEntry::new(KeywordKind::VideoCodec),

    // ── Video color depth ────────────────────────────────────────
    "8BIT" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "8-BIT" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "10BIT" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "10-BIT" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "10BITS" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "HI10" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "HI10P" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "HI444" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "HI444P" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "HI444PP" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "12BIT" => KeywordEntry::new(KeywordKind::VideoColorDepth),
    "12-BIT" => KeywordEntry::new(KeywordKind::VideoColorDepth),

    // ── Video dynamic range ──────────────────────────────────────
    "HDR" => KeywordEntry::new(KeywordKind::VideoDynamicRange),
    "HDR10" => KeywordEntry::new(KeywordKind::VideoDynamicRange),
    "HDR10+" => KeywordEntry::new(KeywordKind::VideoDynamicRange),
    "DOLBY VISION" => KeywordEntry::new(KeywordKind::VideoDynamicRange),
    "DV" => KeywordEntry::ambiguous(KeywordKind::VideoDynamicRange),
    "SDR" => KeywordEntry::new(KeywordKind::VideoDynamicRange),
    "HLG" => KeywordEntry::new(KeywordKind::VideoDynamicRange),

    // ── Video frame rate ─────────────────────────────────────────
    "23.976FPS" => KeywordEntry::new(KeywordKind::VideoFrameRate),
    "24FPS" => KeywordEntry::new(KeywordKind::VideoFrameRate),
    "29.97FPS" => KeywordEntry::new(KeywordKind::VideoFrameRate),
    "30FPS" => KeywordEntry::new(KeywordKind::VideoFrameRate),
    "60FPS" => KeywordEntry::new(KeywordKind::VideoFrameRate),
    "120FPS" => KeywordEntry::new(KeywordKind::VideoFrameRate),

    // ── Video terms (general) ────────────────────────────────────
    "REMUX" => KeywordEntry::new(KeywordKind::VideoTerm),
    "RAW" => KeywordEntry::ambiguous(KeywordKind::VideoTerm),

    // ── Audio codecs ─────────────────────────────────────────────
    "AAC" => KeywordEntry::new(KeywordKind::AudioCodec),
    "AACX2" => KeywordEntry::new(KeywordKind::AudioCodec),
    "AACX3" => KeywordEntry::new(KeywordKind::AudioCodec),
    "AACX4" => KeywordEntry::new(KeywordKind::AudioCodec),
    "AC3" => KeywordEntry::new(KeywordKind::AudioCodec),
    "EAC3" => KeywordEntry::new(KeywordKind::AudioCodec),
    "E-AC-3" => KeywordEntry::new(KeywordKind::AudioCodec),
    "FLAC" => KeywordEntry::new(KeywordKind::AudioCodec),
    "FLACX2" => KeywordEntry::new(KeywordKind::AudioCodec),
    "FLACX3" => KeywordEntry::new(KeywordKind::AudioCodec),
    "FLACX4" => KeywordEntry::new(KeywordKind::AudioCodec),
    "MP3" => KeywordEntry::new(KeywordKind::AudioCodec),
    "OGG" => KeywordEntry::new(KeywordKind::AudioCodec),
    "VORBIS" => KeywordEntry::new(KeywordKind::AudioCodec),
    "OPUS" => KeywordEntry::ambiguous(KeywordKind::AudioCodec),
    "DTS" => KeywordEntry::new(KeywordKind::AudioCodec),
    "DTS-HD" => KeywordEntry::new(KeywordKind::AudioCodec),
    "DTS-ES" => KeywordEntry::new(KeywordKind::AudioCodec),
    "TRUEHD" => KeywordEntry::new(KeywordKind::AudioCodec),
    "TRUE-HD" => KeywordEntry::new(KeywordKind::AudioCodec),
    "LPCM" => KeywordEntry::new(KeywordKind::AudioCodec),
    "PCM" => KeywordEntry::new(KeywordKind::AudioCodec),
    "ATMOS" => KeywordEntry::new(KeywordKind::AudioCodec),
    "DOLBY ATMOS" => KeywordEntry::new(KeywordKind::AudioCodec),
    "DTSX" => KeywordEntry::new(KeywordKind::AudioCodec),
    "DTS:X" => KeywordEntry::new(KeywordKind::AudioCodec),

    // ── Audio channels ───────────────────────────────────────────
    "2.0" => KeywordEntry::ambiguous(KeywordKind::AudioChannels),
    "2.0CH" => KeywordEntry::new(KeywordKind::AudioChannels),
    "2CH" => KeywordEntry::new(KeywordKind::AudioChannels),
    "5.1" => KeywordEntry::ambiguous(KeywordKind::AudioChannels),
    "5.1CH" => KeywordEntry::new(KeywordKind::AudioChannels),
    "7.1" => KeywordEntry::ambiguous(KeywordKind::AudioChannels),
    "7.1CH" => KeywordEntry::new(KeywordKind::AudioChannels),
    "MONO" => KeywordEntry::new(KeywordKind::AudioChannels),
    "STEREO" => KeywordEntry::new(KeywordKind::AudioChannels),
    "SURROUND" => KeywordEntry::new(KeywordKind::AudioChannels),

    // ── Audio terms ──────────────────────────────────────────────
    "DUAL AUDIO" => KeywordEntry::new(KeywordKind::AudioTerm),
    "DUALAUDIO" => KeywordEntry::new(KeywordKind::AudioTerm),
    "DUAL-AUDIO" => KeywordEntry::new(KeywordKind::AudioTerm),
    "MULTI-AUDIO" => KeywordEntry::new(KeywordKind::AudioTerm),

    // ── Resolution ───────────────────────────────────────────────
    "480P" => KeywordEntry::new(KeywordKind::Resolution),
    "576P" => KeywordEntry::new(KeywordKind::Resolution),
    "720P" => KeywordEntry::new(KeywordKind::Resolution),
    "1080P" => KeywordEntry::new(KeywordKind::Resolution),
    "1080I" => KeywordEntry::new(KeywordKind::Resolution),
    "2160P" => KeywordEntry::new(KeywordKind::Resolution),
    "4K" => KeywordEntry::new(KeywordKind::Resolution),
    "UHD" => KeywordEntry::new(KeywordKind::Resolution),
    "SD" => KeywordEntry::ambiguous(KeywordKind::Resolution),
    "HD" => KeywordEntry::ambiguous(KeywordKind::Resolution),
    "FHD" => KeywordEntry::new(KeywordKind::Resolution),
    "QHD" => KeywordEntry::new(KeywordKind::Resolution),

    // ── Source ────────────────────────────────────────────────────
    "BD" => KeywordEntry::ambiguous(KeywordKind::Source),
    "BDMV" => KeywordEntry::new(KeywordKind::Source),
    "BDREMUX" => KeywordEntry::new(KeywordKind::Source),
    "BDRIP" => KeywordEntry::new(KeywordKind::Source),
    "BD-RIP" => KeywordEntry::new(KeywordKind::Source),
    "BLURAY" => KeywordEntry::new(KeywordKind::Source),
    "BLU-RAY" => KeywordEntry::new(KeywordKind::Source),
    "DVD" => KeywordEntry::new(KeywordKind::Source),
    "DVD5" => KeywordEntry::new(KeywordKind::Source),
    "DVD9" => KeywordEntry::new(KeywordKind::Source),
    "DVDRIP" => KeywordEntry::new(KeywordKind::Source),
    "DVD-RIP" => KeywordEntry::new(KeywordKind::Source),
    "DVDREMUX" => KeywordEntry::new(KeywordKind::Source),
    "R2DVD" => KeywordEntry::new(KeywordKind::Source),
    "HDTV" => KeywordEntry::new(KeywordKind::Source),
    "TV" => KeywordEntry::ambiguous(KeywordKind::Source),
    "TVRIP" => KeywordEntry::new(KeywordKind::Source),
    "TV-RIP" => KeywordEntry::new(KeywordKind::Source),
    "WEB" => KeywordEntry::ambiguous(KeywordKind::Source),
    "WEBDL" => KeywordEntry::new(KeywordKind::Source),
    "WEB-DL" => KeywordEntry::new(KeywordKind::Source),
    "WEBRIP" => KeywordEntry::new(KeywordKind::Source),
    "WEB-RIP" => KeywordEntry::new(KeywordKind::Source),
    "HDCAM" => KeywordEntry::new(KeywordKind::Source),
    "TS" => KeywordEntry::ambiguous(KeywordKind::Source),
    "BATCH" => KeywordEntry::new(KeywordKind::Source),
    "VHS" => KeywordEntry::new(KeywordKind::Source),
    "VHSRIP" => KeywordEntry::new(KeywordKind::Source),
    "LASERDISC" => KeywordEntry::new(KeywordKind::Source),
    "LD" => KeywordEntry::ambiguous(KeywordKind::Source),
    "LDRIP" => KeywordEntry::new(KeywordKind::Source),

    // ── Streaming sources ────────────────────────────────────────
    "ABEMA" => KeywordEntry::new(KeywordKind::StreamingSource),
    "AMZN" => KeywordEntry::new(KeywordKind::StreamingSource),
    "AMAZON" => KeywordEntry::new(KeywordKind::StreamingSource),
    "B-GLOBAL" => KeywordEntry::new(KeywordKind::StreamingSource),
    "BILIBILI" => KeywordEntry::new(KeywordKind::StreamingSource),
    "BAHA" => KeywordEntry::new(KeywordKind::StreamingSource),
    "CR" => KeywordEntry::ambiguous(KeywordKind::StreamingSource),
    "CRUNCHYROLL" => KeywordEntry::new(KeywordKind::StreamingSource),
    "DSNP" => KeywordEntry::new(KeywordKind::StreamingSource),
    "DISNEY+" => KeywordEntry::new(KeywordKind::StreamingSource),
    "FUNI" => KeywordEntry::new(KeywordKind::StreamingSource),
    "FUNIMATION" => KeywordEntry::new(KeywordKind::StreamingSource),
    "HIDIVE" => KeywordEntry::new(KeywordKind::StreamingSource),
    "HULU" => KeywordEntry::new(KeywordKind::StreamingSource),
    "NF" => KeywordEntry::ambiguous(KeywordKind::StreamingSource),
    "NETFLIX" => KeywordEntry::new(KeywordKind::StreamingSource),
    "VRV" => KeywordEntry::new(KeywordKind::StreamingSource),
    "WAKANIM" => KeywordEntry::new(KeywordKind::StreamingSource),
    "MUSE" => KeywordEntry::ambiguous(KeywordKind::StreamingSource),

    // ── Episode type / anime type ────────────────────────────────
    "SP" => KeywordEntry::ambiguous(KeywordKind::EpisodeType),
    "SPECIAL" => KeywordEntry::new(KeywordKind::EpisodeType),
    "SPECIALS" => KeywordEntry::new(KeywordKind::EpisodeType),
    "OVA" => KeywordEntry::new(KeywordKind::EpisodeType),
    "ONA" => KeywordEntry::new(KeywordKind::EpisodeType),
    "OAD" => KeywordEntry::new(KeywordKind::EpisodeType),
    "OAV" => KeywordEntry::new(KeywordKind::EpisodeType),
    "MOVIE" => KeywordEntry::new(KeywordKind::EpisodeType),
    "GEKIJOUBAN" => KeywordEntry::new(KeywordKind::EpisodeType),
    "ED" => KeywordEntry::ambiguous(KeywordKind::EpisodeType),
    "ENDING" => KeywordEntry::new(KeywordKind::EpisodeType),
    "NCED" => KeywordEntry::new(KeywordKind::EpisodeType),
    "NCOP" => KeywordEntry::new(KeywordKind::EpisodeType),
    "OP" => KeywordEntry::ambiguous(KeywordKind::EpisodeType),
    "OPENING" => KeywordEntry::new(KeywordKind::EpisodeType),
    "PV" => KeywordEntry::ambiguous(KeywordKind::EpisodeType),
    "PREVIEW" => KeywordEntry::new(KeywordKind::EpisodeType),
    "TRAILER" => KeywordEntry::new(KeywordKind::EpisodeType),
    "CM" => KeywordEntry::ambiguous(KeywordKind::EpisodeType),
    "MENU" => KeywordEntry::new(KeywordKind::EpisodeType),
    "EXTRA" => KeywordEntry::new(KeywordKind::EpisodeType),
    "EXTRAS" => KeywordEntry::new(KeywordKind::EpisodeType),
    "OMAKE" => KeywordEntry::new(KeywordKind::EpisodeType),
    "PICTURE DRAMA" => KeywordEntry::new(KeywordKind::EpisodeType),

    // ── Episode prefix keywords ──────────────────────────────────
    "EP" => KeywordEntry::prefix(KeywordKind::Episode),
    "EP." => KeywordEntry::prefix(KeywordKind::Episode),
    "EPS" => KeywordEntry::prefix(KeywordKind::Episode),
    "EPISODE" => KeywordEntry::prefix(KeywordKind::Episode),
    "EPISODES" => KeywordEntry::prefix(KeywordKind::Episode),

    // ── Season prefix keywords ───────────────────────────────────
    "SEASON" => KeywordEntry::prefix(KeywordKind::Season),
    "SAISON" => KeywordEntry::prefix(KeywordKind::Season),

    // ── Part keywords ────────────────────────────────────────────
    "PART" => KeywordEntry::ambiguous(KeywordKind::Part),
    "COUR" => KeywordEntry::ambiguous(KeywordKind::Part),

    // ── Volume prefix keywords ───────────────────────────────────
    "VOL" => KeywordEntry::prefix(KeywordKind::Volume),
    "VOL." => KeywordEntry::prefix(KeywordKind::Volume),
    "VOLUME" => KeywordEntry::prefix(KeywordKind::Volume),

    // ── Release version ──────────────────────────────────────────
    "V0" => KeywordEntry::new(KeywordKind::ReleaseVersion),
    "V2" => KeywordEntry::new(KeywordKind::ReleaseVersion),
    "V3" => KeywordEntry::new(KeywordKind::ReleaseVersion),
    "V4" => KeywordEntry::new(KeywordKind::ReleaseVersion),

    // ── Release info ─────────────────────────────────────────────
    "REMASTER" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "REMASTERED" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "UNCENSORED" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "UNCUT" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "DIRECTOR'S CUT" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "PROPER" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "REPACK" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "REVISED" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "COMPLETE" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "FINAL" => KeywordEntry::ambiguous(KeywordKind::ReleaseInfo),
    "PATCHED" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "WIDESCREEN" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "FULLSCREEN" => KeywordEntry::new(KeywordKind::ReleaseInfo),
    "LETTERBOX" => KeywordEntry::new(KeywordKind::ReleaseInfo),

    // ── Subtitles ────────────────────────────────────────────────
    "MULTI-SUB" => KeywordEntry::new(KeywordKind::Subtitles),
    "MULTI-SUBS" => KeywordEntry::new(KeywordKind::Subtitles),
    "MULTISUB" => KeywordEntry::new(KeywordKind::Subtitles),
    "SUBBED" => KeywordEntry::new(KeywordKind::Subtitles),
    "DUBBED" => KeywordEntry::new(KeywordKind::Subtitles),
    "SUB" => KeywordEntry::ambiguous(KeywordKind::Subtitles),
    "SUBS" => KeywordEntry::ambiguous(KeywordKind::Subtitles),
    "DUB" => KeywordEntry::ambiguous(KeywordKind::Subtitles),
    "HARDSUB" => KeywordEntry::new(KeywordKind::Subtitles),
    "SOFTSUB" => KeywordEntry::new(KeywordKind::Subtitles),
    "HARDSUBS" => KeywordEntry::new(KeywordKind::Subtitles),
    "SOFTSUBS" => KeywordEntry::new(KeywordKind::Subtitles),
    "ASS" => KeywordEntry::ambiguous(KeywordKind::Subtitles),
    "SRT" => KeywordEntry::ambiguous(KeywordKind::Subtitles),
    "SSA" => KeywordEntry::ambiguous(KeywordKind::Subtitles),

    // ── Languages ────────────────────────────────────────────────
    "ENG" => KeywordEntry::new(KeywordKind::Language),
    "ENGLISH" => KeywordEntry::new(KeywordKind::Language),
    "JPN" => KeywordEntry::new(KeywordKind::Language),
    "JAPANESE" => KeywordEntry::new(KeywordKind::Language),
    "JAP" => KeywordEntry::new(KeywordKind::Language),
    "CHI" => KeywordEntry::new(KeywordKind::Language),
    "CHINESE" => KeywordEntry::new(KeywordKind::Language),
    "KOR" => KeywordEntry::new(KeywordKind::Language),
    "KOREAN" => KeywordEntry::new(KeywordKind::Language),
    "ESP" => KeywordEntry::new(KeywordKind::Language),
    "SPANISH" => KeywordEntry::new(KeywordKind::Language),
    "FRE" => KeywordEntry::new(KeywordKind::Language),
    "FRENCH" => KeywordEntry::new(KeywordKind::Language),
    "GER" => KeywordEntry::new(KeywordKind::Language),
    "GERMAN" => KeywordEntry::new(KeywordKind::Language),
    "ITA" => KeywordEntry::new(KeywordKind::Language),
    "ITALIAN" => KeywordEntry::new(KeywordKind::Language),
    "POR" => KeywordEntry::new(KeywordKind::Language),
    "PORTUGUESE" => KeywordEntry::new(KeywordKind::Language),
    "RUS" => KeywordEntry::new(KeywordKind::Language),
    "RUSSIAN" => KeywordEntry::new(KeywordKind::Language),
    "ARA" => KeywordEntry::new(KeywordKind::Language),
    "ARABIC" => KeywordEntry::new(KeywordKind::Language),
    "THA" => KeywordEntry::new(KeywordKind::Language),
    "THAI" => KeywordEntry::new(KeywordKind::Language),
    "VIE" => KeywordEntry::new(KeywordKind::Language),
    "VIETNAMESE" => KeywordEntry::new(KeywordKind::Language),
    "IND" => KeywordEntry::ambiguous(KeywordKind::Language),
    "INDONESIAN" => KeywordEntry::new(KeywordKind::Language),
    "MALAY" => KeywordEntry::new(KeywordKind::Language),
    "HIN" => KeywordEntry::new(KeywordKind::Language),
    "HINDI" => KeywordEntry::new(KeywordKind::Language),
    "TAM" => KeywordEntry::new(KeywordKind::Language),
    "TAMIL" => KeywordEntry::new(KeywordKind::Language),
    "TEL" => KeywordEntry::new(KeywordKind::Language),
    "TELUGU" => KeywordEntry::new(KeywordKind::Language),
    "MULTI" => KeywordEntry::ambiguous(KeywordKind::Language),

    // ── Device compatibility ─────────────────────────────────────
    "ANDROID" => KeywordEntry::ambiguous(KeywordKind::DeviceCompat),
    "IPAD3" => KeywordEntry::new(KeywordKind::DeviceCompat),
    "IPHONE5" => KeywordEntry::new(KeywordKind::DeviceCompat),
    "PS3" => KeywordEntry::new(KeywordKind::DeviceCompat),
    "XBOX" => KeywordEntry::new(KeywordKind::DeviceCompat),
    "XBOX360" => KeywordEntry::new(KeywordKind::DeviceCompat),

    // ── File extensions ──────────────────────────────────────────
    "MKV" => KeywordEntry::new(KeywordKind::FileExtension),
    "MP4" => KeywordEntry::new(KeywordKind::FileExtension),
    "AVI" => KeywordEntry::new(KeywordKind::FileExtension),
    "OGM" => KeywordEntry::new(KeywordKind::FileExtension),
    "WMV" => KeywordEntry::new(KeywordKind::FileExtension),
    "MPG" => KeywordEntry::new(KeywordKind::FileExtension),
    "FLV" => KeywordEntry::new(KeywordKind::FileExtension),
    "WEBM" => KeywordEntry::new(KeywordKind::FileExtension),
    "M4V" => KeywordEntry::new(KeywordKind::FileExtension),
    "MOV" => KeywordEntry::new(KeywordKind::FileExtension),
    "3GP" => KeywordEntry::new(KeywordKind::FileExtension),
    "RM" => KeywordEntry::new(KeywordKind::FileExtension),
    "RMVB" => KeywordEntry::new(KeywordKind::FileExtension),
    "M2TS" => KeywordEntry::new(KeywordKind::FileExtension),
};

/// Look up a keyword (case-insensitive), ignoring flags.
/// Preserves backward compatibility with the old API.
pub fn lookup(s: &str) -> Option<KeywordKind> {
    KEYWORDS.get(s.to_uppercase().as_str()).map(|e| e.kind)
}

/// Look up a keyword with contextual matching.
///
/// If `is_enclosed` is false, keywords with the `AMBIGUOUS` flag are skipped.
/// This prevents short/common words like "BD", "SD", "SP" from being matched
/// when they appear in free text (where they're more likely part of a title).
pub fn lookup_contextual(s: &str, is_enclosed: bool) -> Option<&'static KeywordEntry> {
    let entry = KEYWORDS.get(s.to_uppercase().as_str())?;
    if !is_enclosed && entry.flags.contains(KeywordFlags::AMBIGUOUS) {
        return None;
    }
    Some(entry)
}
