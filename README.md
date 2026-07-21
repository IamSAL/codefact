# codefacts

A codebase-knowledge engine. Point it at a repo and describe, in plain language,
what you want to learn. On a schedule it mines a slice of the code with headless
`claude`, builds a persistent **memory graph** of the system (modules, services,
libraries, APIs, integrations, data-flows and their relationships), and pushes the
most relevant, novel insight to **Telegram**. The graph deepens over time, so the
facts get richer and more connected.

Built on the [iii](https://iii.dev) framework — cron, HTTP, state and a full
observability console come from iii. It's a **single self-driving binary**:
`codefacts start` boots the iii engine (auto-configuring every provider) and runs
the worker in-process. No project scaffolding, no `iii worker add` — you only ever
edit settings (repo, senders).

## How it works

```
cron (iii)  ─►  codefacts::tick  ─►  codefacts::mine
                                        │  claude -p (read-only) in your repo
                                        ▼
                        merge into memory graph (iii-state)
codefacts::tick  ─►  codefacts::emit  ─►  rank insights  ─►  Telegram
```

Everything is observable in the iii console (`codefacts console`, http://127.0.0.1:3113):
workers, functions, traces, logs, and the state (graph) browser.

## Install

**Prerequisites:** the [`iii`](https://iii.dev) engine and [Claude Code](https://claude.com/claude-code)
(`claude`) must be installed. `codefacts init` checks for both.

```sh
# Homebrew (macOS/Linux)
brew install codefacts

# cargo
cargo install codefacts-cli

# npm (downloads a prebuilt, checksum-verified binary)
npm i -g codefacts

# curl
curl -fsSL https://raw.githubusercontent.com/OWNER/codefacts/main/packaging/install.sh | sh
```

The optional desktop tray app ships as a separate bundle (Homebrew cask / dmg / msi / AppImage).

## Usage

```sh
codefacts init      # settings only: Telegram token+chat, times, repo path, interest
codefacts start     # does everything: boots the engine + all providers + worker
codefacts mine      # analyze a slice now
codefacts emit      # push one insight now
codefacts console   # open the observability console
codefacts graph     # dump the knowledge graph nodes
codefacts history   # recent facts sent
codefacts status
codefacts stop
codefacts uninstall
```

## Configuration

- Config (non-secret): per-OS config dir, `config.toml`.
- **Secrets** (Telegram token/chat): a separate `secrets.toml` with `0600` perms —
  **never** written to iii-state (the console state browser is visible).
- Memory graph + coverage + emissions: iii-state, scoped per repo
  (`cf:<repo-id>:{nodes,edges,coverage,emissions}`), central per-OS data dir by
  default or in-repo `.codefacts/` (opt-in).

## Build from source

```sh
cargo build --release              # core + the single `codefacts` binary
cargo test --workspace             # unit tests
(cd gui/src-tauri && cargo build)  # optional tray app (Rust side)
cargo tauri icon gui/src-tauri/icons/icon.png   # generate bundle icons, then:
cargo tauri build                  # dmg / msi / AppImage
```

## Layout

```
crates/core    pure, unit-tested knowledge logic (Store/Engine/Sender traits)
crates/cli     the single `codefacts` binary (CLI + in-process worker)
gui/           optional Tauri v2 tray app
packaging/     brew / npm / scoop / winget / install.sh
```
