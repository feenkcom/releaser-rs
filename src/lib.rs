#[macro_use]
extern crate lazy_static;

mod release;
mod version;

pub use release::{GitHub, Release};
pub use version::{Version, VersionBump, VersionBumpParseError};
