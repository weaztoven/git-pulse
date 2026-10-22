use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState, Tabs, Wrap},
    Frame,
};

use crate::{
    app::{ActiveTab, App, RepoEntry},
    github::models::Repo,
};

struct Palette {
    foreground: Color,
    accent: Color,
    border: Color,
    muted: Color,
    highlight: Color,
    warning: Color,
    error: Color,
    success: Color,
}

fn palette(theme: &str) -> Palette {
    match theme.to_ascii_lowercase().as_str() {
        "nord" => Palette {
            foreground: Color::Rgb(216, 222, 233),
            accent: Color::Rgb(136, 192, 208),
            border: Color::Rgb(76, 86, 106),
            muted: Color::Rgb(129, 161, 193),
            highlight: Color::Rgb(59, 66, 82),
            warning: Color::Rgb(235, 203, 139),
            error: Color::Rgb(191, 97, 106),
            success: Color::Rgb(163, 190, 140),
        },
        "dracula" => Palette {
            foreground: Color::Rgb(248, 248, 242),
            accent: Color::Rgb(189, 147, 249),
            border: Color::Rgb(98, 114, 164),
            muted: Color::Rgb(139, 233, 253),
            highlight: Color::Rgb(68, 71, 90),
            warning: Color::Rgb(241, 250, 140),
            error: Color::Rgb(255, 85, 85),
            success: Color::Rgb(80, 250, 123),
        },
        _ => Palette {
            foreground: Color::Rgb(205, 214, 244),
            accent: Color::Rgb(203, 166, 247),
            border: Color::Rgb(88, 91, 112),
            muted: Color::Rgb(147, 153, 178),
            highlight: Color::Rgb(69, 71, 90),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
        },
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let palette = palette(&app.theme);
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_header(frame, app, header, &palette);

    match app.active_tab {
        ActiveTab::Watchlist => {
            if app.repos.is_empty() {
                render_empty(frame, body, &palette);
            } else {
                let [list_area, detail_area] =
                    Layout::vertical([Constraint::Min(4), Constraint::Length(13)]).areas(body);
                render_table(frame, app, list_area, &palette);
                render_details(frame, app.selected_entry(), detail_area, &palette);
            }
        }
        ActiveTab::Trending => render_trending(frame, app, body, &palette),
        ActiveTab::MyRepos => render_my_repos(frame, app, body, &palette),
    }

    render_footer(frame, app, footer, &palette);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect, palette: &Palette) {
    let selected = match app.active_tab {
        ActiveTab::Watchlist => 0,
        ActiveTab::Trending => 1,
        ActiveTab::MyRepos => 2,
    };
    let tabs = Tabs::new(vec![
        Line::from("[1] Watchlist"),
        Line::from("[2] Trending"),
        Line::from("[3] My Repos"),
    ])
    .select(selected)
    .highlight_style(
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
    )
    .style(Style::default().fg(palette.muted))
    .block(
        Block::bordered()
            .border_style(Style::default().fg(palette.border))
            .title(format!(" git-pulse v{} ", env!("CARGO_PKG_VERSION"))),
    );
    frame.render_widget(tabs, area);
}

fn render_empty(frame: &mut Frame, area: Rect, palette: &Palette) {
    let message =
        Paragraph::new("No repositories tracked yet. Add some to the `repos` list in config.toml.")
            .style(Style::default().fg(palette.foreground))
            .block(
                Block::bordered()
                    .border_style(Style::default().fg(palette.border))
                    .title(" Repositories "),
            );
    frame.render_widget(message, area);
}

fn render_table(frame: &mut Frame, app: &mut App, area: Rect, palette: &Palette) {
    let header = Row::new([
        "Repository",
        "Stars",
        "Issues",
        "Release",
        "Last commit",
        "Status",
    ])
    .style(
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .repos
        .iter()
        .map(|entry| repo_row(entry, palette))
        .collect();

    let widths = [
        Constraint::Min(24),
        Constraint::Length(9),
        Constraint::Length(8),
        Constraint::Length(18),
        Constraint::Length(12),
        Constraint::Length(9),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(palette.border))
                .title(" Repositories "),
        )
        .row_highlight_style(
            Style::default()
                .bg(palette.highlight)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.watchlist_table_state);
}

fn render_trending(frame: &mut Frame, app: &mut App, area: Rect, palette: &Palette) {
    if app.trending_repos.is_empty() {
        let message = if app.trending_loading {
            "Loading popular repositories..."
        } else if let Some(error) = &app.trending_error {
            return frame.render_widget(
                Paragraph::new(format!("Could not load popular repositories: {error}"))
                    .style(Style::default().fg(palette.error))
                    .block(
                        Block::bordered()
                            .border_style(Style::default().fg(palette.border))
                            .title(" Trending "),
                    ),
                area,
            );
        } else {
            "No popular repositories found. Press r to retry."
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(palette.foreground))
                .block(
                    Block::bordered()
                        .border_style(Style::default().fg(palette.border))
                        .title(" Trending "),
                ),
            area,
        );
        return;
    }

    let table_area = if let Some(error) = &app.trending_error {
        let [error_area, table_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
        frame.render_widget(
            Paragraph::new(format!("Refresh failed: {error}"))
                .style(Style::default().fg(palette.error))
                .wrap(Wrap { trim: true }),
            error_area,
        );
        table_area
    } else {
        area
    };

    let rows = app.trending_repos.iter().map(|repo| {
        Row::new(vec![
            Cell::from(repo.full_name.clone()),
            Cell::from(format_count(repo.stargazers_count)),
            Cell::from(format_count(repo.open_issues_count)),
            Cell::from(repo.description.clone().unwrap_or_else(|| "-".to_string())),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Min(24),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Percentage(50),
        ],
    )
    .header(
        Row::new(["Repository", "Stars", "Issues", "Description"]).style(
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_style(Style::default().fg(palette.border))
            .title(" Popular public repositories (sorted by stars) "),
    )
    .row_highlight_style(
        Style::default()
            .bg(palette.highlight)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(table, table_area, &mut app.trending_table_state);
}

fn render_my_repos(frame: &mut Frame, app: &mut App, area: Rect, palette: &Palette) {
    if app.my_repos.is_empty() {
        let message = if app.my_repos_loading {
            format!("Loading public repositories for {}...", app.github_username)
        } else if let Some(error) = &app.my_repos_error {
            format!("Could not load My Repos: {error}")
        } else {
            "No public repositories found. Press r to retry.".to_string()
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(
                    Style::default().fg(app
                        .my_repos_error
                        .as_ref()
                        .map_or(palette.foreground, |_| palette.error)),
                )
                .wrap(Wrap { trim: true })
                .block(
                    Block::bordered()
                        .border_style(Style::default().fg(palette.border))
                        .title(" My Repos "),
                ),
            area,
        );
        return;
    }

    let table_area = if let Some(error) = &app.my_repos_error {
        let [error_area, table_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
        frame.render_widget(
            Paragraph::new(format!("Refresh failed: {error}"))
                .style(Style::default().fg(palette.error))
                .wrap(Wrap { trim: true }),
            error_area,
        );
        table_area
    } else {
        area
    };
    let [list_area, details_area] =
        Layout::vertical([Constraint::Min(5), Constraint::Length(8)]).areas(table_area);
    render_public_repo_table(
        frame,
        &app.my_repos,
        &mut app.my_repos_table_state,
        list_area,
        &format!(
            " Public repositories | sort: {} (s to change) ",
            app.my_repos_sort.label()
        ),
        palette,
    );
    let selected = app
        .my_repos_table_state
        .selected()
        .and_then(|index| app.my_repos.get(index));
    render_my_repo_details(frame, selected, details_area, palette);
}

fn render_public_repo_table(
    frame: &mut Frame,
    repos: &[Repo],
    state: &mut TableState,
    area: Rect,
    title: &str,
    palette: &Palette,
) {
    let rows = repos.iter().map(|repo| {
        Row::new(vec![
            Cell::from(repo.full_name.clone()),
            Cell::from(format_count(repo.stargazers_count)),
            Cell::from(format_count(repo.open_issues_count)),
            Cell::from(repo.description.clone().unwrap_or_else(|| "-".to_string())),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Min(24),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Percentage(50),
        ],
    )
    .header(
        Row::new(["Repository", "Stars", "Issues", "Description"]).style(
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_style(Style::default().fg(palette.border))
            .title(title),
    )
    .row_highlight_style(
        Style::default()
            .bg(palette.highlight)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(table, area, state);
}

fn render_my_repo_details(frame: &mut Frame, repo: Option<&Repo>, area: Rect, palette: &Palette) {
    let block = Block::bordered()
        .border_style(Style::default().fg(palette.border))
        .title(" Repository details ");
    let Some(repo) = repo else {
        frame.render_widget(
            Paragraph::new("Select a repository to see its metadata.")
                .style(Style::default().fg(palette.foreground))
                .block(block),
            area,
        );
        return;
    };

    let lines = vec![
        Line::from(Span::styled(
            repo.full_name.clone(),
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )),
        label_line(
            "Language",
            repo.language.clone().unwrap_or_else(|| "-".to_string()),
            palette,
        ),
        label_line("Stars", format_count(repo.stargazers_count), palette),
        label_line("Forks", format_count(repo.forks_count), palette),
        label_line("Open issues", format_count(repo.open_issues_count), palette),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(palette.foreground))
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn repo_row(entry: &RepoEntry, palette: &Palette) -> Row<'static> {
    let (status_text, status_color) = status_of(entry, palette);
    let status = Cell::from(Span::styled(status_text, Style::default().fg(status_color)));

    match &entry.info {
        Some(info) => {
            let release = info
                .latest_release
                .as_ref()
                .map_or_else(|| "-".to_string(), |release| release.tag_name.clone());
            let commit_date = info.latest_commit.as_ref().map_or_else(
                || "-".to_string(),
                |commit| commit.commit.author.date.format("%Y-%m-%d").to_string(),
            );

            Row::new(vec![
                Cell::from(info.repo.full_name.clone()),
                Cell::from(format_count(info.repo.stargazers_count)),
                Cell::from(format_count(info.repo.open_issues_count)),
                Cell::from(release),
                Cell::from(commit_date),
                status,
            ])
        }
        None => Row::new(vec![
            Cell::from(entry.name.clone()),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
            status,
        ]),
    }
}

fn status_of(entry: &RepoEntry, palette: &Palette) -> (&'static str, Color) {
    if entry.loading {
        ("loading", palette.warning)
    } else if entry.error.is_some() {
        ("error", palette.error)
    } else if entry.info.is_some() {
        ("ok", palette.success)
    } else {
        ("idle", palette.muted)
    }
}

fn render_details(frame: &mut Frame, entry: Option<&RepoEntry>, area: Rect, palette: &Palette) {
    let block = Block::bordered()
        .border_style(Style::default().fg(palette.border))
        .title(" Details ");

    let Some(entry) = entry else {
        frame.render_widget(
            Paragraph::new("Select a repository to see its details.").block(block),
            area,
        );
        return;
    };

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        entry.name.clone(),
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
    ))];

    if let Some(info) = &entry.info {
        lines.push(label_line("URL", info.repo.html_url.clone(), palette));
        lines.push(label_line(
            "About",
            info.repo
                .description
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            palette,
        ));

        let release = match &info.latest_release {
            Some(release) => {
                let date = release
                    .published_at
                    .map_or_else(|| "-".to_string(), |d| d.format("%Y-%m-%d").to_string());
                format!("{} ({})", release.tag_name, date)
            }
            None => "none".to_string(),
        };
        lines.push(label_line("Release", release, palette));

        let commit = match &info.latest_commit {
            Some(commit) => format!(
                "{} {} - {} ({})",
                commit.short_sha(),
                commit.summary(),
                commit.commit.author.name,
                commit.commit.author.date.format("%Y-%m-%d %H:%M UTC"),
            ),
            None => "none".to_string(),
        };
        lines.push(label_line("Commit", commit, palette));
        lines.push(Line::from(Span::styled(
            "Recent commits",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )));
        for commit in info.commit_history.iter().take(5) {
            lines.push(Line::from(format!(
                "{} {} — {} ({})",
                commit.short_sha(),
                commit.summary(),
                commit.commit.author.name,
                commit.commit.author.date.format("%Y-%m-%d"),
            )));
        }
    } else if entry.loading {
        lines.push(Line::from("Loading..."));
    }

    if let Some(updated) = entry.last_updated {
        lines.push(label_line(
            "Updated",
            updated.format("%H:%M:%S").to_string(),
            palette,
        ));
    }

    if let Some(error) = &entry.error {
        lines.push(Line::from(Span::styled(
            format!("Error: {error}"),
            Style::default().fg(palette.error),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(palette.foreground))
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn label_line(label: &str, value: String, palette: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<9}"), Style::default().fg(palette.accent)),
        Span::raw(value),
    ])
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect, palette: &Palette) {
    if let Some((message, _)) = &app.status_message {
        frame.render_widget(
            Paragraph::new(format!("{message}   Tab: switch tab")).style(
                Style::default()
                    .fg(palette.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            area,
        );
    } else {
        let state = if app.is_loading() {
            "refreshing..."
        } else {
            "idle"
        };
        let tab_shortcuts = match app.active_tab {
            ActiveTab::MyRepos => "   s: sort   i: issues   p: pulls",
            _ => "",
        };
        let sort = match app.active_tab {
            ActiveTab::MyRepos => format!("   sort: {}", app.my_repos_sort.label()),
            _ => String::new(),
        };
        let text = format!(
            "q: quit   Tab: switch tab   ↑/↓ j/k: select   Enter: open   c: copy clone   r: refresh{tab_shortcuts}{sort}   theme: {}   auto-refresh: {}s   {}",
            app.theme, app.refresh_interval_secs, state
        );
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(palette.muted)),
            area,
        );
    }
}

fn format_count(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(ch);
    }
    formatted
}
