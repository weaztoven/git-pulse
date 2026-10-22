use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Repo {
    pub full_name: String,
    pub description: Option<String>,
    pub stargazers_count: u64,
    pub open_issues_count: u64,
    pub html_url: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub forks_count: u64,
    #[serde(default)]
    pub pushed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub license: Option<RepoLicense>,
}

#[derive(Debug, Clone)]
pub struct TrendingResults {
    pub repos: Vec<Repo>,
    pub rate_limit_remaining: Option<String>,
    pub rate_limit_limit: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoLicense {
    pub name: Option<String>,
    pub spdx_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitItem {
    pub sha: String,
    pub commit: CommitDetail,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitDetail {
    pub message: String,
    pub author: CommitAuthor,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub date: DateTime<Utc>,
}

impl CommitItem {
    pub fn short_sha(&self) -> &str {
        self.sha.get(..7).unwrap_or(&self.sha)
    }

    pub fn summary(&self) -> &str {
        self.commit.message.lines().next().unwrap_or("")
    }
}

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub repo: Repo,
    pub latest_release: Option<Release>,
    pub latest_commit: Option<CommitItem>,
    pub commit_history: Vec<CommitItem>,
}

#[derive(Debug, Clone, Default)]
pub struct RepoEtags {
    pub repo: Option<String>,
    pub release: Option<String>,
    pub commit: Option<String>,
}

#[derive(Debug)]
pub enum Conditional<T> {
    NotModified,
    Updated {
        value: Option<T>,
        etag: Option<String>,
    },
}

#[derive(Debug)]
pub enum FetchResult {
    NotModified,
    Updated {
        repo: Conditional<Repo>,
        latest_release: Conditional<Release>,
        commit_history: Conditional<Vec<CommitItem>>,
    },
}
