use anyhow::anyhow;
use chrono::{DateTime, Utc};
use clap::Parser;
use dateparser::DateTimeUtc;
use dirs::config_dir;
use humantime::Duration;
use octocrab::auth::OAuth;
use octocrab::models::issues::Issue;
use octocrab::models::pulls::PullRequest;
use octocrab::Octocrab;
use repo_activity_summary::{Activity, Event, RepoRef, TimeRange};
use serde::Deserialize;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration as StdDuration;

#[derive(Clone, Debug)]
enum TimeOrDuration {
    DateTime(DateTimeUtc),
    Ago(Duration),
}

impl FromStr for TimeOrDuration {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.strip_suffix(" ago") {
            Some(ago) => Self::Ago(Duration::from_str(ago)?),
            None => Self::DateTime(DateTimeUtc::from_str(s)?),
        })
    }
}

impl From<TimeOrDuration> for DateTime<Utc> {
    fn from(value: TimeOrDuration) -> Self {
        match value {
            TimeOrDuration::DateTime(t) => t.0,
            TimeOrDuration::Ago(duration) => || -> Option<DateTime<Utc>> {
                let duration: StdDuration = duration.into();
                let duration = chrono::Duration::from_std(duration).ok()?;
                Utc::now().checked_sub_signed(duration)
            }()
            .unwrap_or(DateTime::<Utc>::MIN_UTC),
        }
    }
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(long)]
    owner: String,

    #[clap(long)]
    repo: String,

    #[clap(long)]
    after: Option<TimeOrDuration>,

    #[clap(long)]
    before: Option<TimeOrDuration>,
}

#[derive(Debug, Deserialize)]
struct GhOAuth {
    user: String,
    oauth_token: String,
    git_protocol: String,
}

#[derive(Debug, Deserialize)]
struct GhHosts {
    #[serde(alias = "github.com")]
    github: GhOAuth,
}

fn gh_oauth() -> anyhow::Result<OAuth> {
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let time_range = TimeRange {
        start: args.after.map(DateTime::<Utc>::from),
        end: args.before.map(DateTime::<Utc>::from),
    };
    let octocrab = Octocrab::builder().oauth(gh_oauth()?).build()?;
    let repo = RepoRef {
        octocrab: octocrab,
        owner: args.owner,
        repo: args.repo,
    };
    PullRequest::list_events_between(&repo, &[Event::Open, Event::Merge], &time_range).await?;
    Issue::list_events_between(&repo, &[Event::Open, Event::Close], &time_range).await?;
    Ok(())
}
