#!/usr/bin/env bash
set -euo pipefail
app=${1:?Usage: check-cli.sh EXECUTABLE}
root=$(cd "$(dirname "$0")/../.." && pwd)
version=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$root/package.json")
test "$("$app" --version)" = "Office363 $version"
"$app" --help | tee /tmp/office363-help.txt
grep -q 'Usage:' /tmp/office363-help.txt
"$app" --check | tee /tmp/office363-check.txt
grep -q 'ready=true' /tmp/office363-check.txt
