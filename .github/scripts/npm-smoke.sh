#!/usr/bin/env bash
# Installs a packed federation-composition-wasm tarball on its own, in an empty project with the
# given graphql major as its peer dependency, and runs the package's smoke test (by package
# name, through "exports") plus a CommonJS require() of it.
#
#   .github/scripts/npm-smoke.sh path/to/federation-composition-wasm-X.Y.Z.tgz 16|17
set -euo pipefail

tarball="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
graphql="$2"
repo="$(cd "$(dirname "$0")/../.." && pwd)"
dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT

cp "$repo/npm/test/smoke.test.mjs" "$dir/"
cd "$dir"
echo '{ "name": "smoke", "private": true, "type": "module" }' > package.json
npm install --no-audit --no-fund --no-package-lock "$tarball" "graphql@$graphql"
echo "node $(node --version), graphql $(node -p 'require("graphql").version')"
node --test smoke.test.mjs
# CommonJS consumers: require() of an ES module (Node 22.12 and later).
node --input-type=commonjs -e '
const m = require("federation-composition-wasm");
if (typeof m.composeServices !== "function") throw new Error("require() did not load the package");
console.log("require() ok");
'
