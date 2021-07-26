use semver::Version as SemverVersion;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum VersionBump {
    Major,
    Minor,
    Patch,
}

#[derive(Clone, Debug)]
pub struct Version(SemverVersion);

impl Version {
    pub fn parse(version: impl Into<String>) -> Result<Self, Box<dyn Error>> {
        let version: String = version.into();
        let version = version.trim_start_matches('v');
        match SemverVersion::parse(&version) {
            Ok(version) => Ok(Self::forced(version.major, version.minor, version.patch)),
            Err(error) => Err(Box::new(error)),
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
        Version(SemverVersion::new(major, minor, patch))
    }

    pub fn bump(&self, bump: VersionBump) -> Self {
        match bump {
            VersionBump::Major => Self::forced(self.major() + 1, 0, 0),
            VersionBump::Minor => Self::forced(self.major(), self.minor() + 1, 0),
            VersionBump::Patch => Self::forced(self.major(), self.minor(), self.patch() + 1),
        }
    }

    pub fn major(&self) -> u64 {
        self.0.major
    }

    pub fn minor(&self) -> u64 {
        self.0.minor
    }

    pub fn patch(&self) -> u64 {
        self.0.patch
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
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Version::parse(s)
    }
}

pub struct VersionBumpParseError(String);

impl Error for VersionBumpParseError {}

impl fmt::Display for VersionBumpParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not parse the bump value {}", self.0.as_str())
    }
}

impl fmt::Debug for VersionBumpParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{ value: {}, file: {}, line: {} }}",
            self.0.as_str(),
            file!(),
            line!()
        )
    }
}

impl FromStr for VersionBump {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version = s.to_string();
        match version.to_lowercase().as_str() {
            "major" => Ok(VersionBump::Major),
            "minor" => Ok(VersionBump::Minor),
            "patch" => Ok(VersionBump::Patch),
            &_ => Err(Box::new(VersionBumpParseError(version))),
        }
    }
}
