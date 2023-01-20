use async_trait::async_trait;
use chrono::{DateTime, Utc};
use octocrab::{
    models::{
        issues::{Issue, IssueStateReason},
        pulls::PullRequest,
    },
    params::State,
    Octocrab, Page,
};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use url::Url;

pub mod auth;

pub struct RepoRef {
    pub octocrab: Octocrab,
    pub owner: String,
    pub repo: String,
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Open,
    Update,
    Close,
    Merge,
}

impl Event {
    pub fn name(&self) -> &'static str {
        use Event::*;
        match self {
            Open => "open",
            Update => "update",
            Close => "close",
            Merge => "merge",
        }
    }

    pub fn past_tense_suffix(&self) -> &'static str {
        if self.name().ends_with("e") {
            "d"
        } else {
            "ed"
        }
    }
}

pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait Activity: Sized + DeserializeOwned + Debug {
    fn name() -> &'static str;

    /// Check if the [`Activity`] is unique,
    /// i.e., if it is not the same as another type of [`Activity`].
    fn is_unique(&self) -> bool;

    fn number(&self) -> u64;

    fn author(&self) -> &str;

    fn title(&self) -> &str;

    fn url(&self) -> &Url;

    fn event_time(&self, event: Event) -> Option<&DateTime<Utc>>;

    async fn list_paged(repo: &RepoRef) -> octocrab::Result<Page<Self>>;

    async fn list(repo: &RepoRef) -> octocrab::Result<Vec<Self>> {
        let page = Self::list_paged(repo).await?;
        let mut all = repo.octocrab.all_pages(page).await?;
        all.retain(Self::is_unique);
        Ok(all)
    }

    fn event_between(&self, event: Event, time_range: &TimeRange) -> bool {
        let time = match self.event_time(event) {
            None => return false,
            Some(time) => time,
        };
        if let Some(start) = time_range.start {
            if time <= &start {
                return false;
            }
        }
        if let Some(end) = time_range.end {
            if time >= &end {
                return false;
            }
        }
        true
    }

    async fn list_events_between(
        repo: &RepoRef,
        events: &[Event],
        time_range: &TimeRange,
    ) -> octocrab::Result<()> {
        let activities = Self::list(&repo).await?;
        for event in events {
            let activities = activities
                .iter()
                .filter(|activity| activity.event_between(*event, time_range))
                .collect::<Vec<_>>();
            println!(
                "{} {}s {}{}",
                activities.len(),
                Self::name(),
                event.name(),
                event.past_tense_suffix(),
            );
            for activity in &activities {
                let time = activity
                    .event_time(*event)
                    .expect("must have an Event to be between")
                    .naive_local();
                println!(
                    "\t#{} ({}{} {}) by {}: {}",
                    activity.number(),
                    event.name(),
                    event.past_tense_suffix(),
                    time,
                    activity.author(),
                    activity.title(),
                );
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Activity for PullRequest {
    fn name() -> &'static str {
        "PR"
    }

    fn is_unique(&self) -> bool {
        true
    }

    fn number(&self) -> u64 {
        self.number
    }

    fn author(&self) -> &str {
        self.user
            .as_ref()
            .map(|user| user.login.as_str())
            .unwrap_or_default()
    }

    fn title(&self) -> &str {
        self.title
            .as_ref()
            .map(|title| title.as_str())
            .unwrap_or_default()
    }

    fn url(&self) -> &Url {
        self.html_url.as_ref().unwrap()
    }

    fn event_time(&self, event: Event) -> Option<&DateTime<Utc>> {
        match event {
            Event::Open => self.created_at.as_ref(),
            Event::Update => self.updated_at.as_ref(),
            Event::Close => self.closed_at.as_ref(),
            Event::Merge => self.merged_at.as_ref(),
        }
    }

    async fn list_paged(repo: &RepoRef) -> octocrab::Result<Page<Self>> {
        repo.octocrab
            .pulls(&repo.owner, &repo.repo)
            .list()
            .state(State::All)
            .per_page(u8::MAX)
            .send()
            .await
    }
}

#[async_trait]
impl Activity for Issue {
    fn name() -> &'static str {
        "issue"
    }

    /// The GitHub API counts [`PullRequest`]s as [`Issue`]s, too,
    /// for some reason, so if this [`Issue`] is also a [`PullRequest`],
    /// then it is not unique.
    fn is_unique(&self) -> bool {
        self.pull_request.is_none()
    }

    fn number(&self) -> u64 {
        self.number
    }

    fn author(&self) -> &str {
        &self.user.login
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn url(&self) -> &Url {
        &self.html_url
    }

    fn event_time(&self, event: Event) -> Option<&DateTime<Utc>> {
        match event {
            Event::Open => Some(&self.created_at),
            Event::Update => Some(&self.updated_at),
            Event::Close => self.closed_at.as_ref(),
            Event::Merge => self
                .closed_at
                .as_ref()
                .filter(|_| self.state_reason == Some(IssueStateReason::Completed)),
        }
    }

    async fn list_paged(repo: &RepoRef) -> octocrab::Result<Page<Self>> {
        repo.octocrab
            .issues(&repo.owner, &repo.repo)
            .list()
            .state(State::All)
            .per_page(u8::MAX)
            .send()
            .await
    }
}
