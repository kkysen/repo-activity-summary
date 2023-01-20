use std::path::{Path, PathBuf};

use dirs::config_dir;
use octocrab::auth::OAuth;
use serde::Deserialize;
use anyhow::anyhow;

#[derive(Debug, Deserialize)]
pub struct GhOAuth {
    pub user: String,
    pub oauth_token: String,
    pub git_protocol: String,
}

#[derive(Debug, Deserialize)]
pub struct GhHosts {
    #[serde(alias = "github.com")]
    pub github: GhOAuth,
}

pub fn gh_oauth() -> anyhow::Result<OAuth> {
    let config = config_dir().ok_or_else(|| anyhow!("no config dir"))?;

    let try_with_dir = |dir: &str| -> anyhow::Result<Vec<u8>> {
        let hosts_path = [config.as_path(), Path::new(dir), Path::new("hosts.yml")]
            .into_iter()
            .collect::<PathBuf>();
        let hosts_bytes = fs_err::read(hosts_path)?;
        Ok(hosts_bytes)
    };

    let mut errors = Vec::new();
    for dir in ["gh", "GitHub CLI"] {
        match try_with_dir(dir) {
            Ok(hosts_bytes) => {
                let hosts = serde_yaml::from_slice::<GhHosts>(&hosts_bytes)?;
                return Ok(OAuth {
                    access_token: hosts.github.oauth_token.parse().unwrap(),
                    token_type: "bearer".into(),
                    scope: vec!["repo".into()],
                });
            }
            Err(e) => errors.push(e),
        }
    }
    Err(anyhow!("{errors:?}"))
}
