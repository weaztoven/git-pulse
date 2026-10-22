[![Crates.io](https://img.shields.io/crates/v/git-pulse.svg)](https://crates.io/crates/git-pulse)
[![Crates.io Downloads](https://img.shields.io/crates/d/git-pulse.svg)](https://crates.io/crates/git-pulse)
[![Crates.io License](https://img.shields.io/crates/l/git-pulse.svg)](https://crates.io/crates/git-pulse)

# git-pulse

**A fast, Rust-powered GitHub TUI tracker with ETag optimization.**

`git-pulse` lets you follow repositories, discover popular projects, and browse
public repositories for a GitHub user—all from your terminal. Public GitHub API
access works without a token or OAuth sign-in.

## What's New in v0.2.0

- **Split-layout interface:** Repository tables take 60% of the main content
  area, with a 40% side panel for selected-repository details. Rounded borders
  give the interface a consistent look.
- **Trending time ranges:** Switch between All-Time, Today, and This Week with
  `t`. Today uses GitHub's UTC calendar-day search qualifier; it is not an exact
  rolling 24-hour window. This Week searches activity from the past seven days.
- **Live summary widgets:** The Trending view summarizes its top language,
  most-starred repository, and GitHub API rate-limit/status information.
- **Readable metrics:** Large counts use compact formatting (for example,
  `555k` or `1.2m`) alongside star, fork, and issue icons and language labels.

## Features

- **Three tabs:** Keep a personal **Watchlist**, browse **Trending** popular
  repositories with time-based filtering, and explore **My Repos** for a
  configured public GitHub username.
- **ETag optimization:** Conditional `If-None-Match` requests reuse unchanged
  repository, release, and commit data to reduce unnecessary downloads and
  help conserve API rate limits.
- **Repository details and commit history:** Inspect repository metadata and
  recent commits in the side panel.
- **Quick integrations:** Open repositories in your default browser, copy
  `git clone` commands to the clipboard, and jump to Issues or Pull Requests.
- **My Repos sorting:** Sort by recent updates, stars, or name.
- **Themes:** Choose Catppuccin, Nord, or Dracula in `config.toml`.
- **Asynchronous refresh:** Network requests run without blocking the terminal
  interface.
- **No token or OAuth required:** Public repository features work using local
  configuration and unauthenticated GitHub API access.

> **About Trending:** Results come from GitHub's Search API, not GitHub's
> official Trending page. All-Time ranks popular repositories by stars; Today
> and This Week filter repositories by recent push activity.

## Requirements

- Rust stable toolchain (Rust 2021 edition)
- Network access to the GitHub API

## Installation

### Install with Cargo

```sh
cargo install git-pulse
```

### Build from source

```sh
git clone https://github.com/weaztoven/git-pulse.git
cd git-pulse
cargo build --release
```

The optimized executable is created at `target/release/git-pulse` (on Windows:
`target\release\git-pulse.exe`). Start the TUI with:

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

- **Windows:** `%APPDATA%\git-pulse\config.toml`
- **macOS:** `~/Library/Application Support/git-pulse/config.toml`
- **Linux:** `$XDG_CONFIG_HOME/git-pulse/config.toml` (usually
  `~/.config/git-pulse/config.toml`)

Example configuration:

```toml
[github]
# Optional. Enables My Repos for this public GitHub account.
username = "weaztoven"

# Repositories to show in Watchlist, in owner/repository format.
repos = [
  "weaztoven/cleaner-cli",
  "rust-lang/rust",
  "ratatui/ratatui",
]

# Optional; defaults to https://api.github.com.
api_url = "https://api.github.com"

[ui]
# Refresh period in seconds. Unauthenticated refreshes use at least 300 seconds.
refresh_interval_secs = 300

# Supported values: "catppuccin", "nord", "dracula".
theme = "catppuccin"
```

No token is needed for public repositories. When using the public API without
authentication, the Watchlist refresh interval is limited to at least 300
seconds.

## Keybindings

| Key | Action |
| --- | --- |
| `Tab` | Cycle through Watchlist, Trending, and My Repos |
| `↑` / `↓`, `j` / `k` | Move the selection in the active list |
| `Enter` | Open the selected repository in the default browser |
| `o` | Open the selected Trending repository in the default browser |
| `c` | Copy the selected repository's `git clone` command |
| `t` | In Trending, cycle time ranges: All-Time → Today → This Week |
| `s` | In My Repos, cycle sorting: updated, stars, name |
| `i` | In My Repos, open the selected repository's Issues page |
| `p` | In My Repos, open the selected repository's Pull Requests page |
| `r` | Refresh the active tab |
| `q` / `Esc` / `Ctrl+C` | Quit |

## License

The package metadata declares the MIT License.
