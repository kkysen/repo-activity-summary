use chrono::{DateTime, Utc};
use clap::Parser;
use dateparser::DateTimeUtc;
use futures::join;
use humantime::Duration;
use octocrab::models::issues::Issue;
use octocrab::models::pulls::PullRequest;
use octocrab::Octocrab;
use repo_activity_summary::auth::gh_oauth;
use repo_activity_summary::{Activity, Event, RepoRef, TimeRange};
use std::fmt::Debug;
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let time_range = TimeRange {
        start: args.after.map(DateTime::<Utc>::from),
        end: args.before.map(DateTime::<Utc>::from),
    };
    let mut parallelize = false;
    let octocrab = {
        let b = Octocrab::builder();
        let b = if let Ok(oauth) = gh_oauth() {
            parallelize = true;
            b.oauth(oauth)
        } else {
            b
        };
        b.build()?
    };
    let repo = RepoRef {
        octocrab,
        parallelize,
        owner: args.owner,
        repo: args.repo,
    };
    let prs = Activity::<PullRequest>::list(&repo);
    let issues = Activity::<Issue>::list(&repo);
    let (prs, issues) = join!(prs, issues);
    let (prs, issues) = (prs?, issues?);
    let prs_opened = prs.events_between(Event::Open, &time_range);
    let prs_merged = prs.events_between(Event::Merge, &time_range);
    let issues_opened = issues.events_between(Event::Open, &time_range);
    let issues_closed = issues.events_between(Event::Close, &time_range);
    println!("{prs_opened}{prs_merged}{issues_opened}{issues_closed}");
    Ok(())
}
