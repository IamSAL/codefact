# Publish codefact to npm (cross-platform)

Layout (esbuild-style): a tiny main package `codefact` + one package per platform
carrying that platform's native binary, wired via `optionalDependencies`. npm
installs only the matching platform package, so `npm i -g codefact` works on any
supported OS/arch.

- `packaging/npm/` → main `codefact` (shim + optional deps)
- `packaging/npm-platforms/darwin-arm64/` → `codefact-darwin-arm64` (macOS arm64 binary)
- `packaging/npm-platforms/linux-x64/` → `codefact-linux-x64` (linux x64 binary)

Installs the commands **`codefacts`** and **`codefact`** (both work).

## Runtime prerequisites (on any machine that RUNS it)

```sh
curl -fsSL https://iii.dev/install.sh | sh   # the iii engine (native binary)
claude --version && claude login             # Claude Code
```

## Auth once (granular token — avoids a 2FA prompt on every publish)

npmjs.com → avatar → **Access Tokens** → **Generate New Token** → **Granular** →
Permissions **Packages and scopes: Read and write** → generate, copy `npm_…`.

```sh
npm config set //registry.npmjs.org/:_authToken npm_XXXXXXXX
```

(Or skip this and append `--otp=<6-digit-code>` to each publish below.)

## Publish (platform packages first, then main)

```sh
cd ~/Work/codefacts/packaging/npm-platforms/darwin-arm64 && npm publish --access public
cd ../linux-x64                                          && npm publish --access public
cd ../../npm                                             && npm publish --access public
```

## Install + try

```sh
npm i -g codefact          # pulls the matching platform binary automatically
codefact init             # Telegram token+chat, repo, interest, times
codefact start            # boots engine + providers + worker
codefact mine && codefact graph
codefact console          # dashboard @ 127.0.0.1:3113
codefact emit             # push to Telegram (needs a real BotFather token)
```

## Adding more platforms later

Build the binary for the target (`cross`/Docker/CI), add
`packaging/npm-platforms/<os>-<arch>/` with a matching `package.json`
(`os`/`cpu` set) + the `codefact` binary, add it to the main package's
`optionalDependencies`, bump versions, and publish. Shim resolves
`codefact-${process.platform}-${process.arch}` automatically.
