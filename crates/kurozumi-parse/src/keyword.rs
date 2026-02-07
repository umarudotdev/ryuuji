use phf::phf_map;

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
}

/// Compile-time keyword lookup table.
/// All keys are UPPERCASE for case-insensitive matching.
pub static KEYWORDS: phf::Map<&'static str, KeywordKind> = phf_map! {
    // Video codecs
    "H264" => KeywordKind::VideoCodec,
    "H.264" => KeywordKind::VideoCodec,
    "X264" => KeywordKind::VideoCodec,
    "H265" => KeywordKind::VideoCodec,
    "H.265" => KeywordKind::VideoCodec,
    "X265" => KeywordKind::VideoCodec,
    "HEVC" => KeywordKind::VideoCodec,
    "AVC" => KeywordKind::VideoCodec,
    "AV1" => KeywordKind::VideoCodec,
    "XVID" => KeywordKind::VideoCodec,
    "DIVX" => KeywordKind::VideoCodec,
    "VP9" => KeywordKind::VideoCodec,
    "10BIT" => KeywordKind::VideoCodec,
    "10-BIT" => KeywordKind::VideoCodec,
    "HI10P" => KeywordKind::VideoCodec,

    // Audio codecs
    "AAC" => KeywordKind::AudioCodec,
    "AC3" => KeywordKind::AudioCodec,
    "EAC3" => KeywordKind::AudioCodec,
    "FLAC" => KeywordKind::AudioCodec,
    "MP3" => KeywordKind::AudioCodec,
    "OGG" => KeywordKind::AudioCodec,
    "VORBIS" => KeywordKind::AudioCodec,
    "OPUS" => KeywordKind::AudioCodec,
    "DTS" => KeywordKind::AudioCodec,
    "TRUEHD" => KeywordKind::AudioCodec,
    "LPCM" => KeywordKind::AudioCodec,

    // Audio terms
    "2.0CH" => KeywordKind::AudioTerm,
    "2CH" => KeywordKind::AudioTerm,
    "5.1" => KeywordKind::AudioTerm,
    "5.1CH" => KeywordKind::AudioTerm,
    "7.1" => KeywordKind::AudioTerm,
    "7.1CH" => KeywordKind::AudioTerm,
    "DUAL AUDIO" => KeywordKind::AudioTerm,
    "DUALAUDIO" => KeywordKind::AudioTerm,

    // Resolution
    "480P" => KeywordKind::Resolution,
    "720P" => KeywordKind::Resolution,
    "1080P" => KeywordKind::Resolution,
    "1080I" => KeywordKind::Resolution,
    "2160P" => KeywordKind::Resolution,
    "4K" => KeywordKind::Resolution,
    "SD" => KeywordKind::Resolution,
    "HD" => KeywordKind::Resolution,

    // Source
    "BD" => KeywordKind::Source,
    "BDREMUX" => KeywordKind::Source,
    "BDRIP" => KeywordKind::Source,
    "BLURAY" => KeywordKind::Source,
    "BLU-RAY" => KeywordKind::Source,
    "DVD" => KeywordKind::Source,
    "DVDRIP" => KeywordKind::Source,
    "DVDREMUX" => KeywordKind::Source,
    "HDTV" => KeywordKind::Source,
    "TV" => KeywordKind::Source,
    "TVRIP" => KeywordKind::Source,
    "WEB" => KeywordKind::Source,
    "WEBDL" => KeywordKind::Source,
    "WEB-DL" => KeywordKind::Source,
    "WEBRIP" => KeywordKind::Source,
    "WEB-RIP" => KeywordKind::Source,
    "HDCAM" => KeywordKind::Source,
    "TS" => KeywordKind::Source,
    "BATCH" => KeywordKind::Source,

    // Video terms
    "HDR" => KeywordKind::VideoTerm,
    "HDR10" => KeywordKind::VideoTerm,
    "HDR10+" => KeywordKind::VideoTerm,
    "DOLBY VISION" => KeywordKind::VideoTerm,
    "DV" => KeywordKind::VideoTerm,

    // Release info
    "REMASTER" => KeywordKind::ReleaseInfo,
    "REMASTERED" => KeywordKind::ReleaseInfo,
    "UNCENSORED" => KeywordKind::ReleaseInfo,
    "UNCUT" => KeywordKind::ReleaseInfo,
    "DIRECTOR'S CUT" => KeywordKind::ReleaseInfo,
    "SPECIAL" => KeywordKind::ReleaseInfo,
    "OVA" => KeywordKind::ReleaseInfo,
    "ONA" => KeywordKind::ReleaseInfo,
    "OAD" => KeywordKind::ReleaseInfo,
    "MULTI-SUB" => KeywordKind::Subtitles,
    "MULTI-SUBS" => KeywordKind::Subtitles,
    "SUBBED" => KeywordKind::Subtitles,
    "DUBBED" => KeywordKind::Subtitles,

    // Languages
    "ENG" => KeywordKind::Language,
    "ENGLISH" => KeywordKind::Language,
    "JPN" => KeywordKind::Language,
    "JAPANESE" => KeywordKind::Language,

    // File extensions (for stripping)
    "MKV" => KeywordKind::FileExtension,
    "MP4" => KeywordKind::FileExtension,
    "AVI" => KeywordKind::FileExtension,
};

/// Look up a keyword (case-insensitive).
pub fn lookup(s: &str) -> Option<KeywordKind> {
    KEYWORDS.get(s.to_uppercase().as_str()).copied()
}
