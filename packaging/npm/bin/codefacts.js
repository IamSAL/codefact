#!/usr/bin/env node
// Exec the bundled native binary, forwarding args + exit code.
const path = require("path");
const { spawnSync } = require("child_process");
const bin = path.join(__dirname, "..", "vendor", "codefacts");
const r = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
process.exit(r.status == null ? 1 : r.status);
