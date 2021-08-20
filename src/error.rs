use thiserror::Error;

pub type Result<T> = core::result::Result<T, ReleaserError>;

#[derive(Error, Debug)]
pub enum ReleaserError {
    #[error("Input/Output error")]
    IoError(#[from] std::io::Error),
    #[error("Octocrab error")]
    OctocrabError(#[from] octocrab::Error),
    #[error("Failed to parse version string")]
    VersionParseError(#[from] semver::Error),
    #[error("Failed to parse url")]
    UrlParseError(#[from] url::ParseError),
    #[error("Failed to perform a request")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to parse version bump {0}")]
    VersionBumpParseError(String),
    #[error("Version or bump are not specified")]
    NoVersionOrBumpError,
}

impl<T> From<ReleaserError> for std::result::Result<T, ReleaserError> {
    fn from(error: ReleaserError) -> Self {
        Err(error)
    }
}
