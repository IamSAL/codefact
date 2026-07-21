# Publish codefacts to npm (quick, single-arch)

This package **bundles the prebuilt native binary** (macOS arm64) so you can
`npm i -g` and try it immediately — no GitHub releases needed. For multi-platform
distribution later, see "Cross-platform" at the bottom.

## 0. Runtime prerequisites (on any machine that will RUN it)

codefacts drives the native `iii` engine and headless `claude`. Both must exist:

```sh
# iii engine (native binary)
curl -fsSL https://iii.dev/install.sh | sh     # or the installer from iii.dev
iii --version

# Claude Code
claude --version && claude login
```

## 1. Pick a name (unscoped `codefacts` is taken on npm)

Edit `package.json` → `"name"`: use a scope you own, e.g.

```json
"name": "@yourNpmUser/codefacts"
```

(Optionally update the `repository` URL.)

## 2. Publish

```sh
cd packaging/npm
npm login
npm publish --access public      # scoped packages need --access public
```

## 3. Install + try (end user)

```sh
npm i -g @yourNpmUser/codefacts

codefacts init        # Telegram token+chat id, repo path, interest, times
codefacts start       # boots the iii engine + all providers + the worker
codefacts mine        # analyze a slice now (runs claude)
codefacts graph       # see the knowledge graph
codefacts console     # observability dashboard @ 127.0.0.1:3113
codefacts emit        # push one insight to Telegram (needs a real bot token)
codefacts status
```

Get a Telegram bot token from **@BotFather** (`/newbot`), send the bot a message,
then get your chat id:
`curl "https://api.telegram.org/bot<TOKEN>/getUpdates"` → `.result[0].message.chat.id`.

## 4. Uninstall

```sh
codefacts uninstall                       # stops daemon, removes login item, purges state
npm rm -g @yourNpmUser/codefacts
```

## Cross-platform (later)

The bundled binary is macOS arm64 only (`os`/`cpu` guards will refuse other
platforms). For all platforms, build per-target binaries in CI
(`.github/workflows/release.yml`) and either:

- publish per-platform optional-dependency packages (esbuild-style), or
- publish a thin wrapper whose `postinstall` downloads the matching
  checksummed binary from GitHub Releases.
