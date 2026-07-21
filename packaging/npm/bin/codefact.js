#!/usr/bin/env node
// Resolve the prebuilt native binary from the matching per-platform package
// (installed via optionalDependencies), then exec it forwarding args + exit code.
const { spawnSync } = require("child_process");
const path = require("path");

const pkg = `codefact-${process.platform}-${process.arch}`;
let bin;
try {
  bin = path.join(
    path.dirname(require.resolve(pkg + "/package.json")),
    "codefact"
  );
} catch {
  console.error(
    `codefact: no prebuilt binary for ${process.platform}-${process.arch}. ` +
      `Supported: darwin-arm64, linux-x64.`
  );
  process.exit(1);
}

const r = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
process.exit(r.status == null ? 1 : r.status);
