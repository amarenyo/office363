#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
output=$(realpath -m "${1:?Usage: prepare-arch.sh OUTPUT_DIRECTORY}")
version=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$root/package.json")
mkdir -p "$output"
git -C "$root" archive --format=tar.gz --prefix="office363-${version}/" HEAD > "$output/office363-${version}.tar.gz"
archive_sha=$(sha256sum "$output/office363-${version}.tar.gz" | cut -d' ' -f1)
sed -e "s/@VERSION@/${version}/g" -e "s/@SOURCE_SHA256@/${archive_sha}/g" \
  "$root/packaging/arch/PKGBUILD.in" > "$output/PKGBUILD"
