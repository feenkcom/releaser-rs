extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

extern crate octocrab as github;
extern crate question;
extern crate reqwest;
extern crate semver;
extern crate serde;
extern crate tokio;
extern crate tokio_util;
extern crate url;

use std::path::PathBuf;

use bytes::{Buf, Bytes};
use clap::{Parser, Subcommand};
use github::models::repos::{Release as OctoRelease, Release};
use github::Octocrab;
use question::{Answer, Question};
use sha256::digest;
use user_error::{UserFacingError, UFE};

pub use error::{ReleaserError, Result};
pub use version::{Version, VersionBump};

mod error;
mod release;
mod version;

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Options {
    /// An owner of the repository
    #[clap(long, required(true))]
    owner: String,
    /// A repository name
    #[clap(long, required(true))]
    repo: String,
    /// A name of the environment variable with personal GitHub token. The reason we do not accept tokens directly is because then it would be exposed in the CI log
    #[clap(long)]
    token: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser, Clone, Debug)]
pub struct ReleaseOptions {
    /// Allow releaser to make decisions without asking
    #[clap(long)]
    auto_accept: bool,
    /// Attach provided assets to the release
    #[clap(long, num_args = 1..)]
    assets: Option<Vec<PathBuf>>,
    #[clap(flatten)]
    next_version: NextVersionOptions,
}

#[derive(Parser, Clone, Debug)]
pub struct NextVersionOptions {
    /// Force a version in form X.Y.Z
    #[arg(long, value_parser = version_parse, conflicts_with_all(&["bump"]))]
    version: Option<Version>,
    /// Component of the version to bump
    #[arg(long, value_enum, conflicts_with_all(&["version"]))]
    bump: Option<VersionBump>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Create a new github release and upload release assets
    Release(ReleaseOptions),
    NextVersion(NextVersionOptions),
}

impl Commands {
    pub async fn run(&self, options: &Options) -> Result<()> {
        match self {
            Commands::Release(release_options) => create_release(options, release_options).await,
            Commands::NextVersion(next_version) => print_next_version(options, next_version).await,
        }
    }
}

fn version_parse(val: &str) -> Result<Version> {
    Version::parse(val)
}

fn create_first_time_version(next_version: &NextVersionOptions) -> Result<Version> {
    if let Some(ref version) = next_version.version {
        Ok(version.clone())
    } else if let Some(ref bump) = next_version.bump {
        Ok(Version::new(bump.clone()))
    } else {
        ReleaserError::NoVersionOrBumpError.into()
    }
}

fn create_next_version(
    current_version: &Version,
    next_version: &NextVersionOptions,
) -> Result<Version> {
    if let Some(ref version) = next_version.version {
        Ok(version.clone())
    } else if let Some(ref bump) = next_version.bump {
        Ok(current_version.bump(bump.clone()))
    } else {
        ReleaserError::NoVersionOrBumpError.into()
    }
}

async fn upload_asset_file(
    file: &PathBuf,
    release: &OctoRelease,
    options: &Options,
    octocrab: &Octocrab,
) -> Result<()> {
    let filename = file.file_name().unwrap().to_str().unwrap();
    let filename_without_extension = file.file_stem().unwrap().to_str().unwrap();

    let file_contents = tokio::fs::read(file).await?;
    let file_data = Bytes::from(file_contents);
    let file_hash = digest(file_data.chunk());

    let asset = octocrab
        .repos(options.owner.clone(), options.repo.clone())
        .releases()
        .upload_asset(release.id.into_inner(), filename, file_data)
        .send()
        .await?;

    info!("Uploaded asset: {:?}", asset);

    let asset_hash = octocrab
        .repos(options.owner.clone(), options.repo.clone())
        .releases()
        .upload_asset(
            release.id.into_inner(),
            &format!("{}.sha256", filename_without_extension),
            Bytes::from(file_hash),
        )
        .send()
        .await?;

    info!("Uploaded asset hash: {:?}", asset_hash);

    Ok(())
}

async fn run() -> Result<()> {
    let options: Options = Options::parse();
    if let Some(ref command) = options.command {
        return command.run(&options).await;
    }
    Ok(())
}

async fn create_release(options: &Options, release_options: &ReleaseOptions) -> Result<()> {
    let octocrab = init_octocrab(options)?;

    let latest_release = get_latest_release(options, &octocrab).await;

    let new_version = match &latest_release {
        None => {
            if !release_options.auto_accept {
                let answer =
                    Question::new("Could not find the latest release. Should we create a new one?")
                        .default(Answer::YES)
                        .show_defaults()
                        .confirm();

                if answer != Answer::YES {
                    return Ok(());
                };
            }
            create_first_time_version(&release_options.next_version)?
        }
        Some(latest_release) => {
            let tag_name = latest_release.tag_name.trim_start_matches('v');
            let current_version = Version::parse(tag_name)?;
            create_next_version(&current_version, &release_options.next_version)?
        }
    };

    // check if the releaser already exists
    let existing_release = octocrab
        .repos(options.owner.clone(), options.repo.clone())
        .releases()
        .get_by_tag(&format!("v{}", &new_version.to_string()))
        .await;

    let new_release = match existing_release {
        Ok(existing_release) => existing_release,
        Err(_) => {
            if !release_options.auto_accept {
                let answer = Question::new(&format!(
                    "Are you sure you want to release a new version {}?",
                    &new_version.to_string()
                ))
                .default(Answer::YES)
                .show_defaults()
                .confirm();

                if answer != Answer::YES {
                    return Ok(());
                };
            }

            let new_release = octocrab
                .repos(options.owner.clone(), options.repo.clone())
                .releases()
                .create(&format!("v{}", &new_version.to_string()))
                .name(&format!("Release v{}", &new_version.to_string()))
                .send()
                .await?;

            println!(
                "A new release version {:?} published!",
                &new_version.to_string()
            );
            new_release
        }
    };

    if let Some(assets) = release_options.assets.as_ref() {
        for asset in assets {
            if !release_options.auto_accept {
                let answer =
                    Question::new(&format!("Should asset be uploaded {}?", asset.display()))
                        .default(Answer::YES)
                        .show_defaults()
                        .confirm();

                if answer != Answer::YES {
                    return Ok(());
                };
            }

            upload_asset_file(asset, &new_release, options, &octocrab).await?;
            println!(
                "Successfully uploaded {} as a release asset",
                asset.display()
            );
        }
    }

    Ok(())
}

fn init_octocrab(options: &Options) -> Result<Octocrab> {
    let mut octocrab_builder = Octocrab::builder();
    if let Some(personal_token) = options
        .token
        .as_ref()
        .map(|var_name| std::env::var(var_name).map_or(None, |token| Some(token)))
        .map_or(None, |token| token)
    {
        octocrab_builder = octocrab_builder.personal_token(personal_token);
    }
    let octocrab = octocrab_builder.build()?;
    Ok(octocrab)
}

async fn print_next_version(
    options: &Options,
    next_version_options: &NextVersionOptions,
) -> Result<()> {
    let octocrab = init_octocrab(options)?;

    let latest_release = get_latest_release(options, &octocrab).await;

    let new_version = match &latest_release {
        None => create_first_time_version(next_version_options)?,
        Some(latest_release) => {
            let tag_name = latest_release.tag_name.trim_start_matches('v');
            let current_version = Version::parse(tag_name)?;
            create_next_version(&current_version, next_version_options)?
        }
    };

    println!("{}", new_version.to_string());
    Ok(())
}

async fn get_latest_release(options: &Options, octocrab: &Octocrab) -> Option<Release> {
    octocrab
        .repos(options.owner.clone(), options.repo.clone())
        .releases()
        .get_latest()
        .await
        .map_or(None, |release| Some(release))
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let error: Box<dyn std::error::Error> = Box::new(error);
        let user_facing_error: UserFacingError = error.into();
        user_facing_error.help("").print_and_exit();
    }
}
