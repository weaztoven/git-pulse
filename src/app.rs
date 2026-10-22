use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    config::Config,
    event::{Event, TICK_RATE},
    github::{
        client::GithubClient,
        models::{
            Conditional, FetchResult, Repo, RepoEtags, RepoInfo, TrendingRange, TrendingResults,
        },
    },
};

pub struct RepoEntry {
    pub name: String,
    pub info: Option<RepoInfo>,
    pub error: Option<String>,
    pub loading: bool,
    pub last_updated: Option<DateTime<Local>>,
    pub etags: RepoEtags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Watchlist,
    Trending,
    MyRepos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MyReposSort {
    Updated,
    Stars,
    Name,
}

impl MyReposSort {
    fn next(self) -> Self {
        match self {
            Self::Updated => Self::Stars,
            Self::Stars => Self::Name,
            Self::Name => Self::Updated,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Updated => "updated",
            Self::Stars => "stars",
            Self::Name => "name",
        }
    }
}

pub struct App {
    pub running: bool,
    pub active_tab: ActiveTab,
    pub repos: Vec<RepoEntry>,
    pub trending_repos: Vec<Repo>,
    pub trending_loading: bool,
    pub trending_error: Option<String>,
    pub trending_rate_limit: Option<(String, String)>,
    pub trending_range: TrendingRange,
    pub my_repos: Vec<Repo>,
    pub my_repos_loading: bool,
    pub my_repos_error: Option<String>,
    pub my_repos_sort: MyReposSort,
    pub watchlist_table_state: TableState,
    pub trending_table_state: TableState,
    pub my_repos_table_state: TableState,
    pub refresh_interval_secs: u64,
    pub status_message: Option<(String, Instant)>,
    pub theme: String,
    pub github_username: String,
    tick_count: u64,
    ticks_per_refresh: u64,
    client: GithubClient,
    tx: UnboundedSender<Event>,
}

impl App {
    pub fn new(config: &Config, client: GithubClient, tx: UnboundedSender<Event>) -> Self {
        let mut repos: Vec<RepoEntry> = Vec::new();
        for name in &config.github.repos {
            let name = name.trim();
            if name.is_empty() || repos.iter().any(|entry| entry.name == name) {
                continue;
            }
            repos.push(RepoEntry {
                name: name.to_string(),
                info: None,
                error: None,
                loading: false,
                last_updated: None,
                etags: RepoEtags::default(),
            });
        }

        let has_token = config
            .github
            .token
            .as_deref()
            .is_some_and(|token| !token.trim().is_empty());
        let minimum_interval_secs = if has_token { 30 } else { 300 };
        let refresh_interval_secs = config.ui.refresh_interval_secs.max(minimum_interval_secs);
        let ticks_per_refresh =
            (refresh_interval_secs * 1000 / TICK_RATE.as_millis() as u64).max(1);

        let mut watchlist_table_state = TableState::default();
        if !repos.is_empty() {
            watchlist_table_state.select(Some(0));
        }

        Self {
            running: true,
            active_tab: ActiveTab::Watchlist,
            repos,
            trending_repos: Vec::new(),
            trending_loading: false,
            trending_error: None,
            trending_rate_limit: None,
            trending_range: TrendingRange::AllTime,
            my_repos: Vec::new(),
            my_repos_loading: false,
            my_repos_error: None,
            my_repos_sort: MyReposSort::Updated,
            watchlist_table_state,
            trending_table_state: TableState::default(),
            my_repos_table_state: TableState::default(),
            refresh_interval_secs,
            status_message: None,
            theme: config.ui.theme.clone(),
            github_username: config.github.username.clone(),
            tick_count: 0,
            ticks_per_refresh,
            client,
            tx,
        }
    }

    pub fn on_tick(&mut self) {
        self.tick_count += 1;
        if self
            .status_message
            .as_ref()
            .is_some_and(|(_, created_at)| created_at.elapsed() > Duration::from_secs(3))
        {
            self.status_message = None;
        }
        if self.tick_count % self.ticks_per_refresh == 0 {
            self.refresh_all();
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false
            }
            KeyCode::Tab => self.switch_tab(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Enter => self.open_selected_repo(),
            KeyCode::Char('c') => self.copy_selected_clone_command(),
            KeyCode::Char('r') => match self.active_tab {
                ActiveTab::Watchlist => self.refresh_all(),
                ActiveTab::Trending => self.refresh_trending(),
                ActiveTab::MyRepos => self.refresh_my_repos(),
            },
            KeyCode::Char('o') if self.active_tab == ActiveTab::Trending => {
                self.open_selected_repo()
            }
            KeyCode::Char('t') if self.active_tab == ActiveTab::Trending => {
                self.cycle_trending_range()
            }
            KeyCode::Char('s') if self.active_tab == ActiveTab::MyRepos => {
                self.cycle_my_repos_sort()
            }
            KeyCode::Char('i') if self.active_tab == ActiveTab::MyRepos => {
                self.open_selected_my_repo_page("issues")
            }
            KeyCode::Char('p') if self.active_tab == ActiveTab::MyRepos => {
                self.open_selected_my_repo_page("pulls")
            }
            _ => {}
        }
    }

    fn cycle_my_repos_sort(&mut self) {
        self.my_repos_sort = self.my_repos_sort.next();
        self.sort_my_repos();
    }

    fn sort_my_repos(&mut self) {
        let selected_name = self
            .my_repos_table_state
            .selected()
            .and_then(|index| self.my_repos.get(index))
            .map(|repo| repo.full_name.clone());
        match self.my_repos_sort {
            MyReposSort::Updated => self.my_repos.sort_by(|left, right| {
                right
                    .pushed_at
                    .or(right.updated_at)
                    .cmp(&left.pushed_at.or(left.updated_at))
            }),
            MyReposSort::Stars => self
                .my_repos
                .sort_by(|left, right| right.stargazers_count.cmp(&left.stargazers_count)),
            MyReposSort::Name => self.my_repos.sort_by(|left, right| {
                left.full_name
                    .to_lowercase()
                    .cmp(&right.full_name.to_lowercase())
            }),
        }
        if let Some(selected_name) = selected_name {
            let selected_index = self
                .my_repos
                .iter()
                .position(|repo| repo.full_name == selected_name);
            self.my_repos_table_state.select(selected_index);
        } else if !self.my_repos.is_empty() {
            self.my_repos_table_state.select(Some(0));
        }
    }

    fn open_selected_my_repo_page(&mut self, page: &str) {
        let Some(name) = self
            .my_repos_table_state
            .selected()
            .and_then(|index| self.my_repos.get(index))
            .map(|repo| repo.full_name.clone())
        else {
            self.set_status_message("No repository selected.".to_string());
            return;
        };

        let url = format!("https://github.com/{name}/{page}");
        let message = match open::that(&url) {
            Ok(()) => format!("Opened {url}"),
            Err(error) => format!("Could not open {url}: {error}"),
        };
        self.set_status_message(message);
    }

    pub fn switch_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Watchlist => ActiveTab::Trending,
            ActiveTab::Trending => ActiveTab::MyRepos,
            ActiveTab::MyRepos => ActiveTab::Watchlist,
        };
        match self.active_tab {
            ActiveTab::Watchlist => {}
            ActiveTab::Trending if self.trending_repos.is_empty() => self.refresh_trending(),
            ActiveTab::MyRepos if self.my_repos.is_empty() => self.refresh_my_repos(),
            _ => {}
        }
    }

    pub fn refresh_trending(&mut self) {
        self.trending_loading = true;
        self.trending_error = None;
        let range = self.trending_range;
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = client
                .fetch_trending_repos(range)
                .await
                .map_err(|error| format!("{error:#}"));
            let _ = tx.send(Event::TrendingUpdated { range, result });
        });
    }

    fn cycle_trending_range(&mut self) {
        self.trending_range = match self.trending_range {
            TrendingRange::AllTime => TrendingRange::Daily,
            TrendingRange::Daily => TrendingRange::Weekly,
            TrendingRange::Weekly => TrendingRange::AllTime,
        };
        self.refresh_trending();
    }

    pub fn on_trending_updated(
        &mut self,
        range: TrendingRange,
        result: Result<TrendingResults, String>,
    ) {
        if range != self.trending_range {
            return;
        }
        self.trending_loading = false;
        match result {
            Ok(results) => {
                let repos = results.repos;
                if !repos.is_empty() && self.trending_table_state.selected().is_none() {
                    self.trending_table_state.select(Some(0));
                } else if self
                    .trending_table_state
                    .selected()
                    .is_some_and(|index| index >= repos.len())
                {
                    self.trending_table_state.select(repos.len().checked_sub(1));
                }
                self.trending_repos = repos;
                self.trending_rate_limit =
                    results.rate_limit_remaining.zip(results.rate_limit_limit);
                self.trending_error = None;
            }
            Err(error) => self.trending_error = Some(error),
        }
    }

    pub fn refresh_my_repos(&mut self) {
        if self.my_repos_loading {
            return;
        }
        let username = self.github_username.trim();
        if username.is_empty() {
            self.my_repos_error =
                Some("Set github.username in config.toml to load public repositories.".into());
            return;
        }

        self.my_repos_loading = true;
        self.my_repos_error = None;
        let username = username.to_string();
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = client
                .fetch_user_repos(&username)
                .await
                .map_err(|error| format!("{error:#}"));
            let _ = tx.send(Event::MyReposUpdated(result));
        });
    }

    pub fn on_my_repos_updated(&mut self, result: Result<Vec<Repo>, String>) {
        self.my_repos_loading = false;
        match result {
            Ok(repos) => {
                if !repos.is_empty() && self.my_repos_table_state.selected().is_none() {
                    self.my_repos_table_state.select(Some(0));
                } else if self
                    .my_repos_table_state
                    .selected()
                    .is_some_and(|index| index >= repos.len())
                {
                    self.my_repos_table_state.select(repos.len().checked_sub(1));
                }
                self.my_repos = repos;
                self.sort_my_repos();
                self.my_repos_error = None;
            }
            Err(error) => self.my_repos_error = Some(error),
        }
    }

    pub fn on_repo_updated(&mut self, name: &str, result: Result<FetchResult, String>) {
        let Some(entry) = self.repos.iter_mut().find(|entry| entry.name == name) else {
            return;
        };

        entry.loading = false;
        match result {
            Ok(FetchResult::NotModified) => {
                if entry.info.is_some() {
                    entry.error = None;
                    entry.last_updated = Some(Local::now());
                } else {
                    entry.error = Some("GitHub returned 304 without cached repository data".into());
                }
            }
            Ok(FetchResult::Updated {
                repo,
                latest_release,
                commit_history,
            }) => {
                let previous = entry.info.take();
                let repo = Self::apply_conditional(
                    repo,
                    previous.as_ref().map(|info| info.repo.clone()),
                    &mut entry.etags.repo,
                );
                let latest_release = Self::apply_conditional(
                    latest_release,
                    previous
                        .as_ref()
                        .and_then(|info| info.latest_release.clone()),
                    &mut entry.etags.release,
                );
                let commit_history = Self::apply_conditional(
                    commit_history,
                    previous.as_ref().map(|info| info.commit_history.clone()),
                    &mut entry.etags.commit,
                );

                let Some(repo) = repo else {
                    entry.error = Some(format!("Repository {} was not found", entry.name));
                    return;
                };
                let commit_history = commit_history.unwrap_or_default();

                entry.info = Some(RepoInfo {
                    repo,
                    latest_release,
                    latest_commit: commit_history.first().cloned(),
                    commit_history,
                });
                entry.error = None;
                entry.last_updated = Some(Local::now());
            }
            Err(message) => entry.error = Some(message),
        }
    }

    pub fn refresh_all(&mut self) {
        let requests: Vec<(String, RepoEtags)> = self
            .repos
            .iter_mut()
            .filter(|entry| !entry.loading)
            .map(|entry| {
                entry.loading = true;
                (entry.name.clone(), entry.etags.clone())
            })
            .collect();

        for (name, etags) in requests {
            self.spawn_fetch(name, etags);
        }
    }

    pub fn is_loading(&self) -> bool {
        self.repos.iter().any(|entry| entry.loading)
            || self.trending_loading
            || self.my_repos_loading
    }

    pub fn selected_entry(&self) -> Option<&RepoEntry> {
        self.watchlist_table_state
            .selected()
            .and_then(|index| self.repos.get(index))
    }

    fn open_selected_repo(&mut self) {
        let Some(name) = self.selected_repo_name() else {
            self.set_status_message("No repository selected.".to_string());
            return;
        };

        let url = format!("https://github.com/{name}");
        let message = match open::that(&url) {
            Ok(()) => format!("Opened {url}"),
            Err(error) => format!("Could not open {url}: {error}"),
        };
        self.set_status_message(message);
    }

    fn copy_selected_clone_command(&mut self) {
        let Some(name) = self.selected_repo_name() else {
            self.set_status_message("No repository selected.".to_string());
            return;
        };

        let command = format!("git clone https://github.com/{name}.git");
        let message = match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(&command) {
                Ok(()) => format!("Copied: {command}"),
                Err(error) => format!("Could not copy clone command: {error}"),
            },
            Err(error) => format!("Could not access clipboard: {error}"),
        };
        self.set_status_message(message);
    }

    fn selected_repo_name(&self) -> Option<String> {
        match self.active_tab {
            ActiveTab::Watchlist => self.selected_entry().map(|entry| {
                entry
                    .info
                    .as_ref()
                    .map_or_else(|| entry.name.clone(), |info| info.repo.full_name.clone())
            }),
            ActiveTab::Trending => self
                .trending_table_state
                .selected()
                .and_then(|index| self.trending_repos.get(index))
                .map(|repo| repo.full_name.clone()),
            ActiveTab::MyRepos => self
                .my_repos_table_state
                .selected()
                .and_then(|index| self.my_repos.get(index))
                .map(|repo| repo.full_name.clone()),
        }
    }

    fn set_status_message(&mut self, message: String) {
        self.status_message = Some((message, Instant::now()));
    }

    fn spawn_fetch(&self, name: String, etags: RepoEtags) {
        let client = self.client.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let result = client
                .fetch_repo_info(&name, &etags)
                .await
                .map_err(|err| format!("{err:#}"));
            let _ = tx.send(Event::RepoUpdated { name, result });
        });
    }

    fn apply_conditional<T>(
        result: Conditional<T>,
        previous: Option<T>,
        etag: &mut Option<String>,
    ) -> Option<T> {
        match result {
            Conditional::NotModified => previous,
            Conditional::Updated {
                value,
                etag: new_etag,
            } => {
                *etag = new_etag;
                value
            }
        }
    }

    fn select_next(&mut self) {
        let item_count = match self.active_tab {
            ActiveTab::Watchlist => self.repos.len(),
            ActiveTab::Trending => self.trending_repos.len(),
            ActiveTab::MyRepos => self.my_repos.len(),
        };
        if item_count == 0 {
            return;
        }
        let state = match self.active_tab {
            ActiveTab::Watchlist => &mut self.watchlist_table_state,
            ActiveTab::Trending => &mut self.trending_table_state,
            ActiveTab::MyRepos => &mut self.my_repos_table_state,
        };
        let next = match state.selected() {
            Some(index) if index + 1 < item_count => index + 1,
            _ => 0,
        };
        state.select(Some(next));
    }

    fn select_previous(&mut self) {
        let item_count = match self.active_tab {
            ActiveTab::Watchlist => self.repos.len(),
            ActiveTab::Trending => self.trending_repos.len(),
            ActiveTab::MyRepos => self.my_repos.len(),
        };
        if item_count == 0 {
            return;
        }
        let state = match self.active_tab {
            ActiveTab::Watchlist => &mut self.watchlist_table_state,
            ActiveTab::Trending => &mut self.trending_table_state,
            ActiveTab::MyRepos => &mut self.my_repos_table_state,
        };
        let previous = match state.selected() {
            Some(index) if index > 0 => index - 1,
            _ => item_count - 1,
        };
        state.select(Some(previous));
    }
}
