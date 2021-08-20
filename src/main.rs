extern crate clap;
extern crate octocrab as github;
extern crate question;
extern crate reqwest;
extern crate semver;
extern crate serde;
extern crate serde_derive;
extern crate tokio;
extern crate tokio_util;
#[macro_use]
extern crate lazy_static;
extern crate url;

mod error;
mod release;
mod version;

pub use error::{ReleaserError, Result};
pub use release::GitHub;
pub use release::Release;

pub use version::{Version, VersionBump};

use clap::{AppSettings, Clap};
use github::models::repos::Release as OctoRelease;
use github::Octocrab;
use question::{Answer, Question};
use reqwest::Url;
use std::path::PathBuf;
use user_error::{UserFacingError, UFE};

#[derive(Clap, Clone, Debug)]
#[clap(version = "1.0", author = "feenk gmbh <contact@feenk.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct ReleaseOptions {
    /// An owner of the repository
    #[clap(long, required(true))]
    owner: String,
    /// A repository name
    #[clap(long, required(true))]
    repo: String,
    /// A name of the environment variable with personal GitHub token. The reason we do not accept tokens directly is because thne it would be exposed in the CI log
    #[clap(long)]
    token: Option<String>,
    /// Force a version in form X.Y.Z
    #[clap(long, parse(try_from_str = version_parse), conflicts_with_all(&["bump"]))]
    version: Option<Version>,
    /// Component of the version to bump
    #[clap(long, conflicts_with_all(&["version"]), possible_values = VersionBump::variants())]
    bump: Option<VersionBump>,
    /// Allow releaser to make decisions without asking
    #[clap(long)]
    auto_accept: bool,
    /// Attach provided assets to the release
    #[clap(long, parse(from_os_str))]
    assets: Option<Vec<PathBuf>>,
}

fn version_parse(val: &str) -> Result<Version> {
    Version::parse(val)
}

fn create_first_time_version(release_options: &ReleaseOptions) -> Result<Version> {
    if let Some(ref version) = release_options.version {
        Ok(version.clone())
    } else if let Some(ref bump) = release_options.bump {
        Ok(Version::new(bump.clone()))
    } else {
        ReleaserError::NoVersionOrBumpError.into()
    }
}

fn create_next_version(
    current_version: &Version,
    release_options: &ReleaseOptions,
) -> Result<Version> {
    if let Some(ref version) = release_options.version {
        Ok(version.clone())
    } else if let Some(ref bump) = release_options.bump {
        Ok(current_version.bump(bump.clone()))
    } else {
        ReleaserError::NoVersionOrBumpError.into()
    }
}

async fn upload_asset_file(
    file: &PathBuf,
    release: &OctoRelease,
    options: &ReleaseOptions,
    octocrab: &Octocrab,
) -> Result<()> {
    let uploads_url = format!(
        "https://uploads.github.com/repos/{}/{}/releases/{}/assets",
        options.owner.clone(),
        options.repo.clone(),
        release.id
    );

    let base_url = Url::parse(&uploads_url)?;

    let filename = file.file_name().unwrap().to_str().unwrap();
    let mut new_url = base_url.clone();
    new_url.set_query(Some(format!("{}={}", "name", filename).as_str()));

    let file_size = std::fs::metadata(file)?.len();
    let file = tokio::fs::File::open(file).await?;
    let stream = tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new());
    let body = reqwest::Body::wrap_stream(stream);

    let builder = octocrab
        .request_builder(new_url.as_str(), reqwest::Method::POST)
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", file_size.to_string());

    builder.body(body).send().await?;
    Ok(())
}

async fn run() -> Result<()> {
    let release_options: ReleaseOptions = ReleaseOptions::parse();

    let mut octocrab_builder = Octocrab::builder();
    if let Some(personal_token) = release_options
        .token
        .as_ref()
        .map(|var_name| std::env::var(var_name).map_or(None, |token| Some(token)))
        .map_or(None, |token| token)
    {
        octocrab_builder = octocrab_builder.personal_token(personal_token);
    }
    let octocrab = octocrab_builder.build()?;

    let latest_release = octocrab
        .repos(release_options.owner.clone(), release_options.repo.clone())
        .releases()
        .get_latest()
        .await
        .map_or(None, |release| Some(release));

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
            create_first_time_version(&release_options)?
        }
        Some(latest_release) => {
            let tag_name = latest_release.tag_name.trim_start_matches('v');
            let current_version = Version::parse(tag_name)?;
            create_next_version(&current_version, &release_options)?
        }
    };

    // check if the releaser already exists
    let existing_release = octocrab
        .repos(release_options.owner.clone(), release_options.repo.clone())
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
                .repos(release_options.owner.clone(), release_options.repo.clone())
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

            upload_asset_file(asset, &new_release, &release_options, &octocrab).await?;
            println!(
                "Successfully uploaded {} as a release asset",
                asset.display()
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let error: Box<dyn std::error::Error> = Box::new(error);
        let user_facing_error: UserFacingError = error.into();
        user_facing_error.help("").print_and_exit();
    }
}
