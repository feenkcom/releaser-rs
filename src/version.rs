use crate::{ReleaserError, Result};
use semver::Version as SemverVersion;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum VersionBump {
    Major,
    Minor,
    Patch,
}

lazy_static! {
    static ref VERSION_BUMP_VARIANTS: [&'static str; 3] = [
        VersionBump::Major.to_str(),
        VersionBump::Minor.to_str(),
        VersionBump::Patch.to_str(),
    ];
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    pub fn parse(version: impl Into<String>) -> Result<Self> {
        let version: String = version.into();
        let version = version.trim_start_matches('v');
        match SemverVersion::parse(&version) {
            Ok(version) => Ok(Self::forced(version.major, version.minor, version.patch)),
            Err(error) => Into::<ReleaserError>::into(error).into(),
        }
    }

    pub fn new(bump: VersionBump) -> Self {
        match bump {
            VersionBump::Major => Self::forced(1, 0, 0),
            VersionBump::Minor => Self::forced(0, 1, 0),
            VersionBump::Patch => Self::forced(0, 0, 1),
        }
    }

    pub fn forced(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn bump(&self, bump: VersionBump) -> Self {
        match bump {
            VersionBump::Major => Self::forced(self.major() + 1, 0, 0),
            VersionBump::Minor => Self::forced(self.major(), self.minor() + 1, 0),
            VersionBump::Patch => Self::forced(self.major(), self.minor(), self.patch() + 1),
        }
    }

    pub fn major(&self) -> u64 {
        self.major
    }

    pub fn minor(&self) -> u64 {
        self.minor
    }

    pub fn patch(&self) -> u64 {
        self.patch
    }
}

impl Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.major(), formatter)?;
        formatter.write_str(".")?;
        Display::fmt(&self.minor(), formatter)?;
        formatter.write_str(".")?;
        Display::fmt(&self.patch(), formatter)?;
        Ok(())
    }
}

impl FromStr for Version {
    type Err = ReleaserError;

    fn from_str(s: &str) -> Result<Self> {
        Version::parse(s)
    }
}

impl VersionBump {
    pub fn to_str(&self) -> &'static str {
        match self {
            VersionBump::Major => "major",
            VersionBump::Minor => "minor",
            VersionBump::Patch => "patch",
        }
    }

    pub fn variants() -> &'static [&'static str] {
        VERSION_BUMP_VARIANTS.as_ref()
    }
}

impl FromStr for VersionBump {
    type Err = ReleaserError;

    fn from_str(s: &str) -> Result<Self> {
        let version = s.to_string();
        match version.to_lowercase().as_str() {
            "major" => Ok(VersionBump::Major),
            "minor" => Ok(VersionBump::Minor),
            "patch" => Ok(VersionBump::Patch),
            &_ => ReleaserError::VersionBumpParseError(version).into(),
        }
    }
}

impl ToString for VersionBump {
    fn to_string(&self) -> String {
        self.to_str().to_owned()
    }
}
