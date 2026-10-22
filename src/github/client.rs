use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, StatusCode,
};
use serde::de::DeserializeOwned;

use super::models::{CommitItem, Conditional, FetchResult, Release, Repo, RepoEtags};
use crate::config::GithubConfig;

#[derive(serde::Deserialize)]
struct SearchRepositories {
    items: Vec<Repo>,
}

#[derive(Clone)]
pub struct GithubClient {
    http: Client,
    base_url: String,
}

impl GithubClient {
    pub fn new(config: &GithubConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "x-github-api-version",
            HeaderValue::from_static("2022-11-28"),
        );

        let token = config
            .token
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty());

        if let Some(token) = token {
            let mut value = HeaderValue::from_str(&format!("Bearer {token}"))
                .context("The GitHub token contains invalid characters")?;
            value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, value);
        }

        let http = Client::builder()
            .user_agent(concat!("git-pulse/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .timeout(Duration::from_secs(15))
            .build()
            .context("Could not build the HTTP client")?;

        Ok(Self {
            http,
            base_url: config.api_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn fetch_repo_info(&self, full_name: &str, etags: &RepoEtags) -> Result<FetchResult> {
        if !is_valid_repo_name(full_name) {
            bail!("`{full_name}` is not a valid repository name, expected `owner/repo`");
        }

        let repo_path = format!("/repos/{full_name}");
        let release_path = format!("/repos/{full_name}/releases/latest");
        let commits_path = format!("/repos/{full_name}/commits?per_page=10");

        let (repo, release, commits) = tokio::try_join!(
            self.get_json::<Repo>(&repo_path, etags.repo.as_deref()),
            self.get_json::<Release>(&release_path, etags.release.as_deref()),
            self.get_json::<Vec<CommitItem>>(&commits_path, etags.commit.as_deref()),
        )?;

        if matches!(&repo, Conditional::Updated { value: None, .. }) {
            bail!("Repository {full_name} was not found");
        }

        if matches!(&repo, Conditional::NotModified)
            && matches!(&release, Conditional::NotModified)
            && matches!(&commits, Conditional::NotModified)
        {
            return Ok(FetchResult::NotModified);
        }

        Ok(FetchResult::Updated {
            repo,
            latest_release: release,
            commit_history: commits,
        })
    }

    pub async fn fetch_user_repos(&self, username: &str) -> Result<Vec<Repo>> {
        if !is_valid_username(username) {
            bail!("`{username}` is not a valid GitHub username");
        }

        let path = format!("/users/{username}/repos?type=owner&sort=updated&per_page=100");
        match self.get_json::<Vec<Repo>>(&path, None).await? {
            Conditional::Updated {
                value: Some(repos), ..
            } => Ok(repos),
            Conditional::Updated { value: None, .. } => {
                bail!("GitHub user {username} was not found or has no public repositories")
            }
            Conditional::NotModified => {
                bail!("Unexpected 304 response while loading public repositories for {username}")
            }
        }
    }

    pub async fn fetch_trending_repos(&self) -> Result<Vec<Repo>> {
        let url = format!("{}/search/repositories", self.base_url);
        let user_agent = concat!("git-pulse/", env!("CARGO_PKG_VERSION"));
        let response = self
            .http
            .get(&url)
            .header(header::USER_AGENT, user_agent)
            .query(&[
                ("q", "stars:>1000"),
                ("sort", "stars"),
                ("order", "desc"),
                ("per_page", "25"),
            ])
            .send()
            .await
            .with_context(|| format!("Request to {url} failed"))?;

        let status = response.status();
        let rate_limit_remaining = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown")
            .to_owned();
        let body = response
            .text()
            .await
            .with_context(|| format!("Could not read the response from {url}"))?;

        if !status.is_success() {
            let detail = if body.is_empty() {
                "empty response body".to_string()
            } else {
                body.chars().take(512).collect()
            };
            if rate_limit_remaining == "0" {
                bail!("GitHub API rate limit exceeded for {url} (remaining: 0): {detail}");
            }
            bail!(
                "GitHub API returned {status} for {url} (rate limit remaining: {rate_limit_remaining}): {detail}"
            );
        }

        let results: SearchRepositories = serde_json::from_str(&body).with_context(|| {
            let detail: String = body.chars().take(512).collect();
            format!("Could not parse the response from {url}: {detail}")
        })?;
        Ok(results.items)
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        etag: Option<&str>,
    ) -> Result<Conditional<T>> {
        let url = format!("{}{}", self.base_url, path);

        let mut request = self.http.get(&url);
        if let Some(etag) = etag {
            request = request.header(
                header::IF_NONE_MATCH,
                HeaderValue::from_str(etag).context("Stored ETag is not a valid HTTP header")?,
            );
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("Request to {url} failed"))?;

        let status = response.status();

        if status == StatusCode::NOT_MODIFIED {
            return Ok(Conditional::NotModified);
        }

        if status == StatusCode::NOT_FOUND || status == StatusCode::CONFLICT {
            return Ok(Conditional::Updated {
                value: None,
                etag: None,
            });
        }

        if !status.is_success() {
            let rate_limited = response
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|value| value.to_str().ok())
                == Some("0");

            if rate_limited {
                bail!("GitHub API rate limit exceeded, set GITHUB_TOKEN to raise the limit");
            }
            if status == StatusCode::UNAUTHORIZED {
                bail!("GitHub rejected the token (401 Unauthorized)");
            }
            bail!("GitHub API returned {status} for {url}");
        }

        let etag = response
            .headers()
            .get(header::ETAG)
            .map(|value| {
                value
                    .to_str()
                    .context("GitHub returned an invalid ETag header")
                    .map(str::to_owned)
            })
            .transpose()?;

        let body = response
            .json::<T>()
            .await
            .with_context(|| format!("Could not parse the response from {url}"))?;

        Ok(Conditional::Updated {
            value: Some(body),
            etag,
        })
    }
}

fn is_valid_repo_name(name: &str) -> bool {
    let mut parts = name.split('/');
    let (Some(owner), Some(repo), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };

    let is_valid_part = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };

    is_valid_part(owner) && is_valid_part(repo)
}

fn is_valid_username(username: &str) -> bool {
    !username.is_empty()
        && username.len() <= 39
        && !username.starts_with('-')
        && !username.ends_with('-')
        && username
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}
