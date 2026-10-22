mod app;
mod config;
mod event;
mod github;
mod tui;
mod ui;

use anyhow::Result;
use app::App;
use config::Config;
use event::{Event, EventHandler, TICK_RATE};
use github::{
    client::GithubClient,
    models::{Conditional, FetchResult, RepoEtags, RepoInfo},
};
use tui::Tui;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    if std::env::args().nth(1).as_deref() == Some("fetch") {
        return run_fetch(config).await;
    }

    let client = GithubClient::new(&config.github)?;

    let mut terminal = tui::init()?;
    let result = run(&mut terminal, &config, client).await;
    tui::restore()?;

    result
}

async fn run(terminal: &mut Tui, config: &Config, client: GithubClient) -> Result<()> {
    let mut events = EventHandler::new(TICK_RATE);
    let mut app = App::new(config, client, events.sender());
    app.refresh_all();

    while app.running {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        match events.next().await {
            Some(Event::Key(key)) => app.on_key(key),
            Some(Event::Tick) => app.on_tick(),
            Some(Event::Resize) => {}
            Some(Event::RepoUpdated { name, result }) => app.on_repo_updated(&name, result),
            Some(Event::TrendingUpdated { range, result }) => {
                app.on_trending_updated(range, result)
            }
            Some(Event::MyReposUpdated(result)) => app.on_my_repos_updated(result),
            None => break,
        }
    }

    Ok(())
}

async fn run_fetch(config: Config) -> Result<()> {
    if config.github.repos.is_empty() {
        println!(
            "No repositories configured. Add some to {}",
            Config::path()?.display()
        );
        return Ok(());
    }

    let client = GithubClient::new(&config.github)?;

    for name in &config.github.repos {
        match client.fetch_repo_info(name, &RepoEtags::default()).await {
            Ok(FetchResult::NotModified) => {
                println!("{name}\n  unchanged (304 Not Modified)\n")
            }
            Ok(FetchResult::Updated {
                repo:
                    Conditional::Updated {
                        value: Some(repo), ..
                    },
                latest_release,
                commit_history,
            }) => {
                let commit_history = conditional_value(commit_history).unwrap_or_default();
                print_repo_info(&RepoInfo {
                    repo,
                    latest_release: conditional_value(latest_release),
                    latest_commit: commit_history.first().cloned(),
                    commit_history,
                })
            }
            Ok(FetchResult::Updated { .. }) => {
                println!("{name}\n  error: Repository data was not returned\n")
            }
            Err(err) => println!("{name}\n  error: {err:#}\n"),
        }
    }

    Ok(())
}

fn conditional_value<T>(result: Conditional<T>) -> Option<T> {
    match result {
        Conditional::NotModified => None,
        Conditional::Updated { value, .. } => value,
    }
}

fn print_repo_info(info: &RepoInfo) {
    let repo = &info.repo;

    println!("{}", repo.full_name);
    println!("  URL         : {}", repo.html_url);
    println!(
        "  Description : {}",
        repo.description.as_deref().unwrap_or("-")
    );
    println!("  Stars       : {}", repo.stargazers_count);
    println!("  Open issues : {}", repo.open_issues_count);

    match &info.latest_release {
        Some(release) => {
            let date = release
                .published_at
                .map_or_else(|| "-".to_string(), |d| d.format("%Y-%m-%d").to_string());
            println!("  Release     : {} ({})", release.tag_name, date);
        }
        None => println!("  Release     : none"),
    }

    match &info.latest_commit {
        Some(commit) => println!(
            "  Last commit : {} {} by {} ({})",
            commit.short_sha(),
            commit.summary(),
            commit.commit.author.name,
            commit.commit.author.date.format("%Y-%m-%d %H:%M UTC"),
        ),
        None => println!("  Last commit : none"),
    }

    println!();
}
