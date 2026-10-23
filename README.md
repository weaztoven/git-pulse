![git-pulse demo](demo.gif)
<div align="center">

# git-pulse

### ⚡ A fast, keyboard-driven GitHub TUI tracker powered by Rust & Ratatui.

[![Crates.io version](https://img.shields.io/crates/v/git-pulse?logo=rust)](https://crates.io/crates/git-pulse)
[![Crates.io downloads](https://img.shields.io/crates/d/git-pulse)](https://crates.io/crates/git-pulse)
[![MIT license](https://img.shields.io/crates/l/git-pulse)](LICENSE)
[![Rust 2021](https://img.shields.io/badge/Rust-2021%20edition-orange?logo=rust)](https://www.rust-lang.org/)

</div>

`git-pulse` is a terminal dashboard for tracking repositories, exploring
popular projects, and browsing a GitHub user's public repositories. Its
asynchronous interface, compact metrics, and conditional requests keep
repository insights close at hand.

## What's New in v0.4.0

- 🔍 **In-Memory Live Filter (`/`)** — Instantly filter the active list by
  repository name, primary language, or description. Filtering is client-side
  and makes no additional GitHub API requests.
- 🎨 **Custom Theme Engine (`[theme]` & `T`)** — Define a `#RRGGBB` color
  palette in `config.toml`, or cycle through the built-in themes at runtime.

## Features

- 🗂️ **Three tabs:** Watchlist, Trending, and My Repos for a configured public
  GitHub username.
- 📈 **Trending time ranges:** Browse All-Time popular repositories, projects
  pushed today, or projects active in the past week.
- 🧩 **Split-view details:** A 60/40 repository table and detail panel, with
  rounded borders, metadata, and recent commit history.
- ⚡ **ETag optimization:** Conditional `If-None-Match` requests reuse unchanged
  Watchlist repository, release, and commit data.
- 🚦 **Rate-limit booster:** Optionally configure a GitHub Personal Access Token
  for up to 5,000 API requests per hour instead of 60. Public repository
  tracking requires no extra token scopes.
- ⌨️ **Keyboard actions:** Open repositories in your browser, copy clone
  commands, jump to Issues or Pull Requests, refresh lists, and sort My Repos.
- 🔍 **Quick filter:** Search loaded repositories in memory by name, language,
  or description.
- 🎨 **Themes:** Catppuccin, Nord, Dracula, and a configurable custom hex
  palette.
- 📊 **Readable metrics:** Compact star, fork, and issue counts, language
  labels, and live Trending summary cards.
- 🔐 **Public by default:** No token or OAuth sign-in is required for the
  standard public-repository workflow.

> **Trending data:** Results use GitHub's Search API rather than the official
> Trending page. All-Time results are sorted by stars; Today and This Week
> filter by recent push activity. Today's search uses a UTC calendar date, not
> an exact rolling 24-hour window.

## Requirements

- Rust stable toolchain (Rust 2021 edition)
- Network access to the GitHub API

## Installation

### Install from crates.io

```sh
cargo install git-pulse
```

### Build from source

```sh
git clone https://github.com/weaztoven/git-pulse.git
cd git-pulse
cargo build --release
```

The optimized binary is written to `target/release/git-pulse` (Windows:
`target\release\git-pulse.exe`). Launch the TUI with:

```sh
cargo run --release
```

Fetch and print configured Watchlist repositories without starting the TUI:

```sh
cargo run --release -- fetch
```

## Configuration

On first launch, `git-pulse` creates a configuration file in your platform's
user configuration directory:

| Platform | Configuration path |
| --- | --- |
| Windows | `%APPDATA%\git-pulse\config.toml` |
| macOS | `~/Library/Application Support/git-pulse/config.toml` |
| Linux | `$XDG_CONFIG_HOME/git-pulse/config.toml` (usually `~/.config/git-pulse/config.toml`) |

Example `config.toml`:

```toml
[github]
# Optional public account for the My Repos tab.
username = "weaztoven"

# Optional PAT for a higher API rate limit. No scopes are needed for public repos.
# Uncomment and replace with your token. Never commit a real token.
# token = "ghp_your_personal_access_token"

# Watchlist entries use owner/repository format.
repos = [
  "weaztoven/cleaner-cli",
  "rust-lang/rust",
  "ratatui/ratatui",
]

# Optional; defaults to https://api.github.com.
api_url = "https://api.github.com"

[ui]
# Unauthenticated requests use a minimum refresh interval of 300 seconds.
refresh_interval_secs = 300
# Supported: "catppuccin", "nord", "dracula", "custom".
theme = "custom"

[theme]
# Optional six-digit RGB hex colors. Omitted colors use Catppuccin defaults.
primary = "#CBA6F7"
secondary = "#9399B2"
text = "#CDD6F4"
background = "#1E1E2E"
border = "#585B70"
highlight = "#45475A"
```

The `token` value is optional; leave it commented out to use the
unauthenticated public API (60 requests per hour). A configured PAT generally
raises the limit to 5,000 requests per hour and needs no additional scopes for
public repository tracking. Keep real tokens private.

Custom colors are optional and use `#RRGGBB` format. Set `ui.theme = "custom"`
to use them. Press `T` to cycle Catppuccin, Nord, Dracula, and the custom theme
when custom colors are configured.

## Keybindings

| Key | Action |
| --- | --- |
| `Tab` | Cycle through Watchlist, Trending, and My Repos |
| `↑` / `↓`, `j` / `k` | Move the selection in the active list |
| `/` | Open the live filter for the active tab |
| `Enter` while filtering | Keep the filter and close the input bar |
| `Esc` while filtering | Clear the filter and restore the full list |
| `Enter` | Open the selected repository in the default browser |
| `o` | Open the selected Trending repository in the default browser |
| `c` | Copy the selected repository's `git clone` command |
| `t` | Cycle Trending ranges: All-Time → Today → This Week |
| `T` | Cycle built-in themes and configured custom theme |
| `s` | In My Repos, cycle sorting: updated, stars, name |
| `i` | In My Repos, open the selected Issues page |
| `p` | In My Repos, open the selected Pull Requests page |
| `r` | Refresh the active tab |
| `q` / `Esc` / `Ctrl+C` | Quit (`Esc` clears the filter while filtering) |

## License

git-pulse is distributed under the MIT License. See [LICENSE](LICENSE).
