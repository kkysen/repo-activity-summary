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
    pub fn as_str(&self) -> &'static str {
        use Event::*;
        match self {
            Open => "open",
            Update => "update",
            Close => "close",
            Merge => "merge",
        }
    }
}

pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait Activity: Sized + DeserializeOwned {
    fn name() -> &'static str;

    fn number(&self) -> u64;

    fn author(&self) -> &str;

    fn title(&self) -> &str;

    fn event_time(&self, event: Event) -> Option<&DateTime<Utc>>;

    async fn list_paged(repo: &RepoRef) -> octocrab::Result<Page<Self>>;

    async fn list(repo: &RepoRef) -> octocrab::Result<Vec<Self>> {
        let page = Self::list_paged(repo).await?;
        let all = repo.octocrab.all_pages(page).await?;
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
            let e = if event.as_str().ends_with("e") {
                ""
            } else {
                "e"
            };
            println!(
                "{} {}s {}{}d",
                activities.len(),
                Self::name(),
                event.as_str(),
                e
            );
        }
        Ok(())
    }
}

#[async_trait]
impl Activity for PullRequest {
    fn name() -> &'static str {
        "PR"
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

    fn number(&self) -> u64 {
        self.number
    }

    fn author(&self) -> &str {
        &self.user.login
    }

    fn title(&self) -> &str {
        &self.title
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
