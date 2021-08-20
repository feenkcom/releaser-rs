use crate::Result;
use crate::Version;

pub enum Release {
    GitHub(GitHub),
}

impl Release {
    pub async fn version(&self) -> Result<Option<Version>> {
        match self {
            Release::GitHub(github) => github.latest_release_version().await,
        }
    }
}

pub struct GitHub {
    owner: String,
    repo: String,
    token: Option<String>,
}

impl GitHub {
    pub fn new(
        owner: impl Into<String>,
        repo: impl Into<String>,
        token: impl Into<Option<String>>,
    ) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            token: token.into(),
        }
    }

    pub async fn latest_release_version(&self) -> Result<Option<Version>> {
        let mut octocrab_builder = octocrab::Octocrab::builder();
        if let Some(personal_token) = self
            .token
            .as_ref()
            .map(|var_name| std::env::var(var_name).map_or(None, |token| Some(token)))
            .map_or(None, |token| token)
        {
            octocrab_builder = octocrab_builder.personal_token(personal_token);
        }
        let octocrab_api = octocrab_builder.build()?;

        let latest_release = octocrab_api
            .repos(&self.owner, &self.repo)
            .releases()
            .get_latest()
            .await
            .map_or(None, |release| Some(release));

        match latest_release {
            None => Ok(None),
            Some(latest_release) => {
                Version::parse(&latest_release.tag_name).map(|version| Some(version))
            }
        }
    }
}
