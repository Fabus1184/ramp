use std::{collections::HashMap, fmt::Debug, num::NonZeroU32};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Value {
    Binary(Box<[u8]>),
    Boolean(bool),
    Flag,
    Float(f64),
    SignedInt(i64),
    String(String),
    UnsignedInt(u64),
}

impl From<symphonia::core::meta::Value> for Value {
    fn from(value: symphonia::core::meta::Value) -> Self {
        match value {
            symphonia::core::meta::Value::Binary(b) => Self::Binary(b),
            symphonia::core::meta::Value::Boolean(b) => Self::Boolean(b),
            symphonia::core::meta::Value::Flag => Self::Flag,
            symphonia::core::meta::Value::Float(f) => Self::Float(f),
            symphonia::core::meta::Value::SignedInt(i) => Self::SignedInt(i),
            symphonia::core::meta::Value::String(s) => Self::String(s),
            symphonia::core::meta::Value::UnsignedInt(u) => Self::UnsignedInt(u),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Binary(_) => write!(f, "Binary(...)"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Flag => write!(f, "Flag"),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::SignedInt(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "{}", s),
            Value::UnsignedInt(u) => write!(f, "{}", u),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Visual {
    pub media_type: String,
    pub dimensions: Option<Size>,
    pub bits_per_pixel: Option<NonZeroU32>,
    pub color_mode: Option<ColorMode>,
    pub usage: Option<StandardVisualKey>,
    pub tags: Vec<Tag>,
    pub data: Box<[u8]>,
}

impl From<symphonia::core::meta::Visual> for Visual {
    fn from(visual: symphonia::core::meta::Visual) -> Self {
        Self {
            media_type: visual.media_type,
            dimensions: visual.dimensions.map(Into::into),
            bits_per_pixel: visual.bits_per_pixel,
            color_mode: visual.color_mode.map(Into::into),
            usage: visual.usage.map(Into::into),
            tags: visual.tags.into_iter().map(Into::into).collect(),
            data: visual.data,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl From<symphonia::core::meta::Size> for Size {
    fn from(size: symphonia::core::meta::Size) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum ColorMode {
    Discrete,
    Indexed(NonZeroU32),
}

impl From<symphonia::core::meta::ColorMode> for ColorMode {
    fn from(color_mode: symphonia::core::meta::ColorMode) -> Self {
        match color_mode {
            symphonia::core::meta::ColorMode::Discrete => Self::Discrete,
            symphonia::core::meta::ColorMode::Indexed(i) => Self::Indexed(i),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum StandardVisualKey {
    FileIcon,
    OtherIcon,
    FrontCover,
    BackCover,
    Leaflet,
    Media,
    LeadArtistPerformerSoloist,
    ArtistPerformer,
    Conductor,
    BandOrchestra,
    Composer,
    Lyricist,
    RecordingLocation,
    RecordingSession,
    Performance,
    ScreenCapture,
    Illustration,
    BandArtistLogo,
    PublisherStudioLogo,
}

impl From<symphonia::core::meta::StandardVisualKey> for StandardVisualKey {
    fn from(value: symphonia::core::meta::StandardVisualKey) -> Self {
        match value {
            symphonia::core::meta::StandardVisualKey::FileIcon => StandardVisualKey::FileIcon,
            symphonia::core::meta::StandardVisualKey::OtherIcon => StandardVisualKey::OtherIcon,
            symphonia::core::meta::StandardVisualKey::FrontCover => StandardVisualKey::FrontCover,
            symphonia::core::meta::StandardVisualKey::BackCover => StandardVisualKey::BackCover,
            symphonia::core::meta::StandardVisualKey::Leaflet => StandardVisualKey::Leaflet,
            symphonia::core::meta::StandardVisualKey::Media => StandardVisualKey::Media,
            symphonia::core::meta::StandardVisualKey::LeadArtistPerformerSoloist => {
                StandardVisualKey::LeadArtistPerformerSoloist
            }
            symphonia::core::meta::StandardVisualKey::ArtistPerformer => {
                StandardVisualKey::ArtistPerformer
            }
            symphonia::core::meta::StandardVisualKey::Conductor => StandardVisualKey::Conductor,
            symphonia::core::meta::StandardVisualKey::BandOrchestra => {
                StandardVisualKey::BandOrchestra
            }
            symphonia::core::meta::StandardVisualKey::Composer => StandardVisualKey::Composer,
            symphonia::core::meta::StandardVisualKey::Lyricist => StandardVisualKey::Lyricist,
            symphonia::core::meta::StandardVisualKey::RecordingLocation => {
                StandardVisualKey::RecordingLocation
            }
            symphonia::core::meta::StandardVisualKey::RecordingSession => {
                StandardVisualKey::RecordingSession
            }
            symphonia::core::meta::StandardVisualKey::Performance => StandardVisualKey::Performance,
            symphonia::core::meta::StandardVisualKey::ScreenCapture => {
                StandardVisualKey::ScreenCapture
            }
            symphonia::core::meta::StandardVisualKey::Illustration => {
                StandardVisualKey::Illustration
            }
            symphonia::core::meta::StandardVisualKey::BandArtistLogo => {
                StandardVisualKey::BandArtistLogo
            }
            symphonia::core::meta::StandardVisualKey::PublisherStudioLogo => {
                StandardVisualKey::PublisherStudioLogo
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Tag {
    std_key: Option<StandardTagKey>,
    key: String,
    value: Value,
}

impl From<symphonia::core::meta::Tag> for Tag {
    fn from(tag: symphonia::core::meta::Tag) -> Self {
        Self {
            std_key: tag.std_key.map(StandardTagKey::from),
            key: tag.key,
            value: tag.value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, serde::Deserialize, serde::Serialize)]
pub enum StandardTagKey {
    AcoustidFingerprint,
    AcoustidId,
    Album,
    AlbumArtist,
    Arranger,
    Artist,
    Bpm,
    Comment,
    Compilation,
    Composer,
    Conductor,
    ContentGroup,
    Copyright,
    Date,
    Description,
    DiscNumber,
    DiscSubtitle,
    DiscTotal,
    EncodedBy,
    Encoder,
    EncoderSettings,
    EncodingDate,
    Engineer,
    Ensemble,
    Genre,
    IdentAsin,
    IdentBarcode,
    IdentCatalogNumber,
    IdentEanUpn,
    IdentIsrc,
    IdentPn,
    IdentPodcast,
    IdentUpc,
    Label,
    Language,
    License,
    Lyricist,
    Lyrics,
    MediaFormat,
    MixDj,
    MixEngineer,
    Mood,
    MovementName,
    MovementNumber,
    MusicBrainzAlbumArtistId,
    MusicBrainzAlbumId,
    MusicBrainzArtistId,
    MusicBrainzDiscId,
    MusicBrainzGenreId,
    MusicBrainzLabelId,
    MusicBrainzOriginalAlbumId,
    MusicBrainzOriginalArtistId,
    MusicBrainzRecordingId,
    MusicBrainzReleaseGroupId,
    MusicBrainzReleaseStatus,
    MusicBrainzReleaseTrackId,
    MusicBrainzReleaseType,
    MusicBrainzTrackId,
    MusicBrainzWorkId,
    Opus,
    OriginalAlbum,
    OriginalArtist,
    OriginalDate,
    OriginalFile,
    OriginalWriter,
    Owner,
    Part,
    PartTotal,
    Performer,
    Podcast,
    PodcastCategory,
    PodcastDescription,
    PodcastKeywords,
    Producer,
    PurchaseDate,
    Rating,
    ReleaseCountry,
    ReleaseDate,
    Remixer,
    ReplayGainAlbumGain,
    ReplayGainAlbumPeak,
    ReplayGainTrackGain,
    ReplayGainTrackPeak,
    Script,
    SortAlbum,
    SortAlbumArtist,
    SortArtist,
    SortComposer,
    SortTrackTitle,
    TaggingDate,
    TrackNumber,
    TrackSubtitle,
    TrackTitle,
    TrackTotal,
    TvEpisode,
    TvEpisodeTitle,
    TvNetwork,
    TvSeason,
    TvShowTitle,
    Url,
    UrlArtist,
    UrlCopyright,
    UrlInternetRadio,
    UrlLabel,
    UrlOfficial,
    UrlPayment,
    UrlPodcast,
    UrlPurchase,
    UrlSource,
    Version,
    Writer,
}

impl From<symphonia::core::meta::StandardTagKey> for StandardTagKey {
    fn from(value: symphonia::core::meta::StandardTagKey) -> Self {
        match value {
            symphonia::core::meta::StandardTagKey::AcoustidFingerprint => {
                StandardTagKey::AcoustidFingerprint
            }
            symphonia::core::meta::StandardTagKey::AcoustidId => StandardTagKey::AcoustidId,
            symphonia::core::meta::StandardTagKey::Album => StandardTagKey::Album,
            symphonia::core::meta::StandardTagKey::AlbumArtist => StandardTagKey::AlbumArtist,
            symphonia::core::meta::StandardTagKey::Arranger => StandardTagKey::Arranger,
            symphonia::core::meta::StandardTagKey::Artist => StandardTagKey::Artist,
            symphonia::core::meta::StandardTagKey::Bpm => StandardTagKey::Bpm,
            symphonia::core::meta::StandardTagKey::Comment => StandardTagKey::Comment,
            symphonia::core::meta::StandardTagKey::Compilation => StandardTagKey::Compilation,
            symphonia::core::meta::StandardTagKey::Composer => StandardTagKey::Composer,
            symphonia::core::meta::StandardTagKey::Conductor => StandardTagKey::Conductor,
            symphonia::core::meta::StandardTagKey::ContentGroup => StandardTagKey::ContentGroup,
            symphonia::core::meta::StandardTagKey::Copyright => StandardTagKey::Copyright,
            symphonia::core::meta::StandardTagKey::Date => StandardTagKey::Date,
            symphonia::core::meta::StandardTagKey::Description => StandardTagKey::Description,
            symphonia::core::meta::StandardTagKey::DiscNumber => StandardTagKey::DiscNumber,
            symphonia::core::meta::StandardTagKey::DiscSubtitle => StandardTagKey::DiscSubtitle,
            symphonia::core::meta::StandardTagKey::DiscTotal => StandardTagKey::DiscTotal,
            symphonia::core::meta::StandardTagKey::EncodedBy => StandardTagKey::EncodedBy,
            symphonia::core::meta::StandardTagKey::Encoder => StandardTagKey::Encoder,
            symphonia::core::meta::StandardTagKey::EncoderSettings => {
                StandardTagKey::EncoderSettings
            }
            symphonia::core::meta::StandardTagKey::EncodingDate => StandardTagKey::EncodingDate,
            symphonia::core::meta::StandardTagKey::Engineer => StandardTagKey::Engineer,
            symphonia::core::meta::StandardTagKey::Ensemble => StandardTagKey::Ensemble,
            symphonia::core::meta::StandardTagKey::Genre => StandardTagKey::Genre,
            symphonia::core::meta::StandardTagKey::IdentAsin => StandardTagKey::IdentAsin,
            symphonia::core::meta::StandardTagKey::IdentBarcode => StandardTagKey::IdentBarcode,
            symphonia::core::meta::StandardTagKey::IdentCatalogNumber => {
                StandardTagKey::IdentCatalogNumber
            }
            symphonia::core::meta::StandardTagKey::IdentEanUpn => StandardTagKey::IdentEanUpn,
            symphonia::core::meta::StandardTagKey::IdentIsrc => StandardTagKey::IdentIsrc,
            symphonia::core::meta::StandardTagKey::IdentPn => StandardTagKey::IdentPn,
            symphonia::core::meta::StandardTagKey::IdentPodcast => StandardTagKey::IdentPodcast,
            symphonia::core::meta::StandardTagKey::IdentUpc => StandardTagKey::IdentUpc,
            symphonia::core::meta::StandardTagKey::Label => StandardTagKey::Label,
            symphonia::core::meta::StandardTagKey::Language => StandardTagKey::Language,
            symphonia::core::meta::StandardTagKey::License => StandardTagKey::License,
            symphonia::core::meta::StandardTagKey::Lyricist => StandardTagKey::Lyricist,
            symphonia::core::meta::StandardTagKey::Lyrics => StandardTagKey::Lyrics,
            symphonia::core::meta::StandardTagKey::MediaFormat => StandardTagKey::MediaFormat,
            symphonia::core::meta::StandardTagKey::MixDj => StandardTagKey::MixDj,
            symphonia::core::meta::StandardTagKey::MixEngineer => StandardTagKey::MixEngineer,
            symphonia::core::meta::StandardTagKey::Mood => StandardTagKey::Mood,
            symphonia::core::meta::StandardTagKey::MovementName => StandardTagKey::MovementName,
            symphonia::core::meta::StandardTagKey::MovementNumber => StandardTagKey::MovementNumber,
            symphonia::core::meta::StandardTagKey::MusicBrainzAlbumArtistId => {
                StandardTagKey::MusicBrainzAlbumArtistId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzAlbumId => {
                StandardTagKey::MusicBrainzAlbumId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzArtistId => {
                StandardTagKey::MusicBrainzArtistId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzDiscId => {
                StandardTagKey::MusicBrainzDiscId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzGenreId => {
                StandardTagKey::MusicBrainzGenreId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzLabelId => {
                StandardTagKey::MusicBrainzLabelId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzOriginalAlbumId => {
                StandardTagKey::MusicBrainzOriginalAlbumId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzOriginalArtistId => {
                StandardTagKey::MusicBrainzOriginalArtistId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzRecordingId => {
                StandardTagKey::MusicBrainzRecordingId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzReleaseGroupId => {
                StandardTagKey::MusicBrainzReleaseGroupId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzReleaseStatus => {
                StandardTagKey::MusicBrainzReleaseStatus
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzReleaseTrackId => {
                StandardTagKey::MusicBrainzReleaseTrackId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzReleaseType => {
                StandardTagKey::MusicBrainzReleaseType
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzTrackId => {
                StandardTagKey::MusicBrainzTrackId
            }
            symphonia::core::meta::StandardTagKey::MusicBrainzWorkId => {
                StandardTagKey::MusicBrainzWorkId
            }
            symphonia::core::meta::StandardTagKey::Opus => StandardTagKey::Opus,
            symphonia::core::meta::StandardTagKey::OriginalAlbum => StandardTagKey::OriginalAlbum,
            symphonia::core::meta::StandardTagKey::OriginalArtist => StandardTagKey::OriginalArtist,
            symphonia::core::meta::StandardTagKey::OriginalDate => StandardTagKey::OriginalDate,
            symphonia::core::meta::StandardTagKey::OriginalFile => StandardTagKey::OriginalFile,
            symphonia::core::meta::StandardTagKey::OriginalWriter => StandardTagKey::OriginalWriter,
            symphonia::core::meta::StandardTagKey::Owner => StandardTagKey::Owner,
            symphonia::core::meta::StandardTagKey::Part => StandardTagKey::Part,
            symphonia::core::meta::StandardTagKey::PartTotal => StandardTagKey::PartTotal,
            symphonia::core::meta::StandardTagKey::Performer => StandardTagKey::Performer,
            symphonia::core::meta::StandardTagKey::Podcast => StandardTagKey::Podcast,
            symphonia::core::meta::StandardTagKey::PodcastCategory => {
                StandardTagKey::PodcastCategory
            }
            symphonia::core::meta::StandardTagKey::PodcastDescription => {
                StandardTagKey::PodcastDescription
            }
            symphonia::core::meta::StandardTagKey::PodcastKeywords => {
                StandardTagKey::PodcastKeywords
            }
            symphonia::core::meta::StandardTagKey::Producer => StandardTagKey::Producer,
            symphonia::core::meta::StandardTagKey::PurchaseDate => StandardTagKey::PurchaseDate,
            symphonia::core::meta::StandardTagKey::Rating => StandardTagKey::Rating,
            symphonia::core::meta::StandardTagKey::ReleaseCountry => StandardTagKey::ReleaseCountry,
            symphonia::core::meta::StandardTagKey::ReleaseDate => StandardTagKey::ReleaseDate,
            symphonia::core::meta::StandardTagKey::Remixer => StandardTagKey::Remixer,
            symphonia::core::meta::StandardTagKey::ReplayGainAlbumGain => {
                StandardTagKey::ReplayGainAlbumGain
            }
            symphonia::core::meta::StandardTagKey::ReplayGainAlbumPeak => {
                StandardTagKey::ReplayGainAlbumPeak
            }
            symphonia::core::meta::StandardTagKey::ReplayGainTrackGain => {
                StandardTagKey::ReplayGainTrackGain
            }
            symphonia::core::meta::StandardTagKey::ReplayGainTrackPeak => {
                StandardTagKey::ReplayGainTrackPeak
            }
            symphonia::core::meta::StandardTagKey::Script => StandardTagKey::Script,
            symphonia::core::meta::StandardTagKey::SortAlbum => StandardTagKey::SortAlbum,
            symphonia::core::meta::StandardTagKey::SortAlbumArtist => {
                StandardTagKey::SortAlbumArtist
            }
            symphonia::core::meta::StandardTagKey::SortArtist => StandardTagKey::SortArtist,
            symphonia::core::meta::StandardTagKey::SortComposer => StandardTagKey::SortComposer,
            symphonia::core::meta::StandardTagKey::SortTrackTitle => StandardTagKey::SortTrackTitle,
            symphonia::core::meta::StandardTagKey::TaggingDate => StandardTagKey::TaggingDate,
            symphonia::core::meta::StandardTagKey::TrackNumber => StandardTagKey::TrackNumber,
            symphonia::core::meta::StandardTagKey::TrackSubtitle => StandardTagKey::TrackSubtitle,
            symphonia::core::meta::StandardTagKey::TrackTitle => StandardTagKey::TrackTitle,
            symphonia::core::meta::StandardTagKey::TrackTotal => StandardTagKey::TrackTotal,
            symphonia::core::meta::StandardTagKey::TvEpisode => StandardTagKey::TvEpisode,
            symphonia::core::meta::StandardTagKey::TvEpisodeTitle => StandardTagKey::TvEpisodeTitle,
            symphonia::core::meta::StandardTagKey::TvNetwork => StandardTagKey::TvNetwork,
            symphonia::core::meta::StandardTagKey::TvSeason => StandardTagKey::TvSeason,
            symphonia::core::meta::StandardTagKey::TvShowTitle => StandardTagKey::TvShowTitle,
            symphonia::core::meta::StandardTagKey::Url => StandardTagKey::Url,
            symphonia::core::meta::StandardTagKey::UrlArtist => StandardTagKey::UrlArtist,
            symphonia::core::meta::StandardTagKey::UrlCopyright => StandardTagKey::UrlCopyright,
            symphonia::core::meta::StandardTagKey::UrlInternetRadio => {
                StandardTagKey::UrlInternetRadio
            }
            symphonia::core::meta::StandardTagKey::UrlLabel => StandardTagKey::UrlLabel,
            symphonia::core::meta::StandardTagKey::UrlOfficial => StandardTagKey::UrlOfficial,
            symphonia::core::meta::StandardTagKey::UrlPayment => StandardTagKey::UrlPayment,
            symphonia::core::meta::StandardTagKey::UrlPodcast => StandardTagKey::UrlPodcast,
            symphonia::core::meta::StandardTagKey::UrlPurchase => StandardTagKey::UrlPurchase,
            symphonia::core::meta::StandardTagKey::UrlSource => StandardTagKey::UrlSource,
            symphonia::core::meta::StandardTagKey::Version => StandardTagKey::Version,
            symphonia::core::meta::StandardTagKey::Writer => StandardTagKey::Writer,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Song {
    pub standard_tags: HashMap<StandardTagKey, Value>,
    pub other_tags: HashMap<String, Value>,
    pub duration: f32,
}

impl Song {
    pub fn gain_factor(&self) -> Option<f32> {
        self.standard_tags
            .get(&StandardTagKey::ReplayGainTrackGain)
            .map(|x| x.to_string())
            .and_then(|x| x.strip_suffix(" dB").map(|x| x.to_string()))
            .and_then(|x| x.parse::<f32>().ok())
            .map(|x| 10.0f32.powf(x / 20.0))
    }
}
