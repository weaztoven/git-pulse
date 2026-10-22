[![Crates.io](https://img.shields.io/crates/v/git-pulse.svg)](https://crates.io/crates/git-pulse)
[![Crates.io Downloads](https://img.shields.io/crates/d/git-pulse.svg)](https://crates.io/crates/git-pulse)
[![Crates.io License](https://img.shields.io/crates/l/git-pulse.svg)](https://crates.io/crates/git-pulse)
# git-pulse

**A fast, Rust-powered GitHub TUI tracker with ETag optimization.**

`git-pulse` lets you monitor repositories from your terminal. It uses public
GitHub API access by default and requires no token or OAuth sign-in.

## Features

- **No-login by default:** Public repository tracking works without a GitHub
  token or OAuth. Configuration is stored locally.
- **Three tabs:** Watchlist, Trending, and My Repos (public repositories for a
  configured GitHub username).
- **Conditional requests:** Per-endpoint ETags and `If-None-Match` requests
  avoid downloading unchanged repository, release, and commit data.
- **Repository details:** View repository statistics and the latest commit
  history.
- **My Repos tools:** Sort repositories by last push/update, stars, or name
  with `s`. Open Issues with `i` and Pull Requests with `p`.
- **Quick actions:** Open the selected repository with `Enter`; copy its
  `git clone` command with `c`.
- **Themes:** Catppuccin, Nord, and Dracula palettes configured in `config.toml`.
- **Asynchronous refresh:** Network requests run without blocking the terminal
  interface.

> **Trending note:** The Trending tab currently uses GitHub's Search API to
> show popular public repositories (more than 1,000 stars), sorted by star
> count. This is a popular-repositories list, not GitHub's time-windowed
> Trending page.

## Requirements

- Rust toolchain (stable; Rust 2021 edition)
- Network access to the GitHub API

## Installation

Clone the repository and build the optimized executable:

### Via Cargo (Recommended)
```bash
cargo install git-pulse
```
```sh
git clone https://github.com/weaztoven/git-pulse.git
cd git-pulse
cargo build --release
```

The executable is created at `target/release/git-pulse` (on Windows:
`target\release\git-pulse.exe`).

Run the TUI with:

```sh
cargo run --release
```

To fetch and print information for configured Watchlist repositories without
starting the TUI:

```sh
cargo run --release -- fetch
```

## Configuration

On first launch, `git-pulse` creates a default configuration file in your
platform's user configuration directory:

- Windows: `%APPDATA%\git-pulse\config.toml`
- macOS: `~/Library/Application Support/git-pulse/config.toml`
- Linux: `$XDG_CONFIG_HOME/git-pulse/config.toml` (usually `~/.config/...`)

Example:

```toml
[github]
# Optional. Enables the My Repos tab for this public GitHub account.
username = "weaztoven"

# Repositories to show in the Watchlist tab, in owner/repository format.
repos = [
  "rust-lang/rust",
  "ratatui/ratatui",
]

# Optional; defaults to https://api.github.com.
api_url = "https://api.github.com"

[ui]
# Refresh period in seconds. Unauthenticated refreshes are limited to at
# least 300 seconds by the application.
refresh_interval_secs = 300

# Supported values: "catppuccin", "nord", "dracula".
theme = "catppuccin"
```

No token is needed for public repositories. The Watchlist refresh interval is
automatically limited to at least 300 seconds when using the public API without
authentication.

## Keybindings

| Key | Action |
| --- | --- |
| `Tab` | Cycle through Watchlist, Trending, and My Repos |
| `↑` / `↓`, `j` / `k` | Move the selection in the active list |
| `Enter` | Open the selected repository in the system browser |
| `c` | Copy the selected repository's `git clone` command |
| `s` | In My Repos, cycle sorting: updated, stars, name |
| `i` | In My Repos, open the selected repository's Issues page |
| `p` | In My Repos, open the selected repository's Pull Requests page |
| `r` | Refresh the active tab |
| `q` / `Esc` | Quit |
| `Ctrl+C` | Quit |

## License

The package metadata declares the MIT License.
