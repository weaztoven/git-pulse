use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Cell, Paragraph, Row, Table, TableState, Tabs, Wrap},
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
                    Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                        .areas(body);
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
            .border_type(BorderType::Rounded)
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
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(palette.border))
                    .title(" Repositories "),
            );
    frame.render_widget(message, area);
}

fn render_table(frame: &mut Frame, app: &mut App, area: Rect, palette: &Palette) {
    let header = Row::new(["Repository", "★ Stars", "Issues", "Language", "🍴 Forks"]).style(
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
        Constraint::Min(12),
        Constraint::Length(9),
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Length(9),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
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
    let [summary_area, content_area] =
        Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).areas(area);
    render_trending_summary(frame, app, summary_area, palette);

    let [table_area, details_area] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .areas(content_area);
    render_trending_table(frame, app, table_area, palette);
    render_trending_details(frame, app, details_area, palette);
}

fn render_trending_summary(frame: &mut Frame, app: &App, area: Rect, palette: &Palette) {
    let [language_area, starred_area, api_area] = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .areas(area);

    let top_language = app
        .trending_repos
        .iter()
        .filter_map(|repo| repo.language.as_deref())
        .fold(
            std::collections::BTreeMap::<&str, usize>::new(),
            |mut counts, language| {
                *counts.entry(language).or_default() += 1;
                counts
            },
        )
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(language, count)| format!("{language} ({count})"))
        .unwrap_or_else(|| {
            if app.trending_loading {
                "Loading...".into()
            } else {
                "—".into()
            }
        });
    render_summary_card(
        frame,
        language_area,
        &format!("{} · Top Language", app.trending_range.label()),
        top_language,
        palette.accent,
        palette,
    );

    let most_starred = app
        .trending_repos
        .iter()
        .max_by_key(|repo| repo.stargazers_count)
        .map(|repo| {
            format!(
                "{} · ★ {}",
                repo.full_name,
                format_count(repo.stargazers_count)
            )
        })
        .unwrap_or_else(|| {
            if app.trending_loading {
                "Loading...".into()
            } else {
                "—".into()
            }
        });
    render_summary_card(
        frame,
        starred_area,
        "Most Starred Repo",
        most_starred,
        palette.warning,
        palette,
    );

    let api_status = if let Some(error) = &app.trending_error {
        format!("Error: {error}")
    } else if app.trending_loading {
        "Fetching repositories...".to_string()
    } else if let Some((remaining, limit)) = &app.trending_rate_limit {
        format!("{remaining} / {limit} requests remaining")
    } else if app.trending_repos.is_empty() {
        "Not loaded · press r to retry".to_string()
    } else {
        "Loaded · rate limit unavailable".to_string()
    };
    render_summary_card(
        frame,
        api_area,
        "API Rate Limit / Status",
        api_status,
        if app.trending_error.is_some() {
            palette.error
        } else {
            palette.success
        },
        palette,
    );
}

fn render_summary_card(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: String,
    value_color: Color,
    palette: &Palette,
) {
    let card = Paragraph::new(value)
        .style(
            Style::default()
                .fg(value_color)
                .add_modifier(Modifier::BOLD),
        )
        .wrap(Wrap { trim: true })
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border))
                .title(Span::styled(
                    format!(" {title} "),
                    Style::default().fg(palette.muted),
                )),
        );
    frame.render_widget(card, area);
}

fn render_trending_table(frame: &mut Frame, app: &mut App, area: Rect, palette: &Palette) {
    let rows = app.trending_repos.iter().map(|repo| {
        Row::new(vec![
            Cell::from(Span::styled(
                repo.full_name.clone(),
                Style::default().fg(palette.success),
            )),
            Cell::from(Span::styled(
                format!("★ {}", format_count(repo.stargazers_count)),
                Style::default().fg(palette.warning),
            )),
            Cell::from(Span::styled(
                repo.language.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(palette.accent),
            )),
            Cell::from(Span::styled(
                format_count(repo.open_issues_count),
                Style::default().fg(palette.error),
            )),
            Cell::from(Span::styled(
                format!("🍴 {}", format_count(repo.forks_count)),
                Style::default().fg(Color::Blue),
            )),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Min(12),
            Constraint::Length(9),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(9),
        ],
    )
    .header(
        Row::new(["Repository", "★ Stars", "Language", "Issues", "🍴 Forks"]).style(
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border))
            .title(format!(
                " Trending {} · {} repositories · press t to change range ",
                app.trending_range.label(),
                app.trending_repos.len(),
            )),
    )
    .row_highlight_style(
        Style::default()
            .bg(palette.highlight)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(table, area, &mut app.trending_table_state);
}

fn render_trending_details(frame: &mut Frame, app: &App, area: Rect, palette: &Palette) {
    let [details_area, shortcuts_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).areas(area);

    let selected = app
        .trending_table_state
        .selected()
        .and_then(|index| app.trending_repos.get(index));
    let details_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.border))
        .title(" Trending Repository ");

    if let Some(repo) = selected {
        let language = repo.language.as_deref().unwrap_or("Unknown");
        let description = repo
            .description
            .as_deref()
            .unwrap_or("No description provided.");
        let details = vec![
            Line::from(Span::styled(
                format!("📦 {}", repo.full_name),
                Style::default()
                    .fg(palette.success)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("◆ {language}"),
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    format!("★ {}  ", format_count(repo.stargazers_count)),
                    Style::default().fg(palette.warning),
                ),
                Span::styled(
                    format!("🍴 {}  ", format_count(repo.forks_count)),
                    Style::default().fg(Color::Blue),
                ),
                Span::styled(
                    format!("🐛 {}", format_count(repo.open_issues_count)),
                    Style::default().fg(palette.error),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Description",
                Style::default()
                    .fg(palette.muted)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(description.to_string()),
        ];
        frame.render_widget(
            Paragraph::new(details)
                .style(Style::default().fg(palette.foreground))
                .wrap(Wrap { trim: true })
                .block(details_block),
            details_area,
        );
    } else {
        let message = if app.trending_loading {
            "Loading popular repositories..."
        } else if let Some(error) = &app.trending_error {
            error.as_str()
        } else {
            "Select a repository to view details."
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(if app.trending_error.is_some() {
                    palette.error
                } else {
                    palette.muted
                }))
                .wrap(Wrap { trim: true })
                .block(details_block),
            details_area,
        );
    }

    let shortcuts =
        Paragraph::new("[t] Time Range   [c] Copy Clone   [o] Open in Browser   [r] Refresh")
            .style(
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .wrap(Wrap { trim: true })
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(palette.border))
                    .title(" Actions "),
            );
    frame.render_widget(shortcuts, shortcuts_area);
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
                        .border_type(BorderType::Rounded)
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
            Cell::from(Span::styled(
                format!("★ {}", format_count(repo.stargazers_count)),
                Style::default().fg(palette.warning),
            )),
            Cell::from(Span::styled(
                format_count(repo.open_issues_count),
                Style::default().fg(palette.error),
            )),
            Cell::from(Span::styled(
                repo.language.clone().unwrap_or_else(|| "-".to_string()),
                Style::default().fg(palette.accent),
            )),
            Cell::from(Span::styled(
                format!("🍴 {}", format_count(repo.forks_count)),
                Style::default().fg(Color::Blue),
            )),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Min(14),
            Constraint::Length(10),
            Constraint::Length(9),
            Constraint::Length(14),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(["Repository", "★ Stars", "Issues", "Language", "🍴 Forks"]).style(
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
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
        .border_type(BorderType::Rounded)
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
            format!("📦 {}", repo.full_name),
            Style::default()
                .fg(palette.success)
                .add_modifier(Modifier::BOLD),
        )),
        label_line(
            "Language",
            repo.language.clone().unwrap_or_else(|| "-".to_string()),
            palette,
        ),
        metric_line(
            "★ Stars",
            format_count(repo.stargazers_count),
            palette.warning,
            palette,
        ),
        metric_line(
            "🍴 Forks",
            format_count(repo.forks_count),
            Color::Blue,
            palette,
        ),
        metric_line(
            "Issues",
            format_count(repo.open_issues_count),
            palette.error,
            palette,
        ),
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
    match &entry.info {
        Some(info) => Row::new(vec![
            Cell::from(Span::styled(
                info.repo.full_name.clone(),
                Style::default().fg(palette.success),
            )),
            Cell::from(Span::styled(
                format!("★ {}", format_count(info.repo.stargazers_count)),
                Style::default().fg(palette.warning),
            )),
            Cell::from(Span::styled(
                format_count(info.repo.open_issues_count),
                Style::default().fg(palette.error),
            )),
            Cell::from(Span::styled(
                info.repo
                    .language
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                Style::default().fg(palette.accent),
            )),
            Cell::from(Span::styled(
                format!("🍴 {}", format_count(info.repo.forks_count)),
                Style::default().fg(Color::Blue),
            )),
        ]),
        None => Row::new(vec![
            Cell::from(entry.name.clone()),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
        ]),
    }
}

fn render_details(frame: &mut Frame, entry: Option<&RepoEntry>, area: Rect, palette: &Palette) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.border))
        .title(" Repository details ");

    let Some(entry) = entry else {
        frame.render_widget(
            Paragraph::new("Select a repository to view details.")
                .style(Style::default().fg(palette.muted))
                .block(block),
            area,
        );
        return;
    };

    let Some(info) = &entry.info else {
        let message = if entry.loading {
            "Loading repository details...".to_string()
        } else {
            entry.error.as_ref().map_or_else(
                || "Repository details are not available yet.".to_string(),
                |e| format!("Error: {e}"),
            )
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(if entry.error.is_some() {
                    palette.error
                } else {
                    palette.muted
                }))
                .block(block)
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    };

    let license = info
        .repo
        .license
        .as_ref()
        .and_then(|license| license.name.as_ref().or(license.spdx_id.as_ref()))
        .cloned()
        .unwrap_or_else(|| "Not specified".to_string());
    let mut lines = vec![
        Line::from(Span::styled(
            format!("📦 {}", info.repo.full_name),
            Style::default()
                .fg(palette.success)
                .add_modifier(Modifier::BOLD),
        )),
        metric_line(
            "★ Stars",
            format_count(info.repo.stargazers_count),
            palette.warning,
            palette,
        ),
        metric_line(
            "🍴 Forks",
            format_count(info.repo.forks_count),
            Color::Blue,
            palette,
        ),
        metric_line(
            "Issues",
            format_count(info.repo.open_issues_count),
            palette.error,
            palette,
        ),
        metric_line("License", license, palette.foreground, palette),
        metric_line(
            "Language",
            info.repo
                .language
                .clone()
                .unwrap_or_else(|| "Not specified".to_string()),
            palette.accent,
            palette,
        ),
        Line::from(Span::styled(
            "Description",
            Style::default()
                .fg(palette.muted)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(
            info.repo
                .description
                .clone()
                .unwrap_or_else(|| "No description provided.".to_string()),
        ),
    ];

    if !info.commit_history.is_empty() {
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
    }

    if let Some(updated) = entry.last_updated {
        lines.push(Line::from(Span::styled(
            format!("Updated {}", updated.format("%H:%M:%S")),
            Style::default().fg(palette.muted),
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

fn metric_line(label: &str, value: String, color: Color, palette: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<12}"),
            Style::default()
                .fg(palette.muted)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, Style::default().fg(color)),
    ])
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
    if value >= 1_000_000_000 {
        return format_compact(value, 1_000_000_000, "b");
    }
    if value >= 1_000_000 {
        return format_compact(value, 1_000_000, "m");
    }
    if value >= 1_000 {
        return format_compact(value, 1_000, "k");
    }

    let digits = value.to_string();
    digits
}

fn format_compact(value: u64, divisor: u64, suffix: &str) -> String {
    let whole = value / divisor;
    let tenths = (value % divisor) * 10 / divisor;
    if whole < 10 && tenths > 0 {
        format!("{whole}.{tenths}{suffix}")
    } else {
        format!("{whole}{suffix}")
    }
}
