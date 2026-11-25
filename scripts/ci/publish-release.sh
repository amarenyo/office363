#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
assets=$(realpath "${1:?Usage: publish-release.sh RELEASE_ASSET_DIRECTORY}")
version=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$root/package.json")
python3 "$root/scripts/ci/check-release.py"
(cd "$assets" && sha256sum --check SHA256SUMS)
notes=$(mktemp)
trap 'rm -f "$notes"' EXIT
cat > "$notes" <<EOF
## Linux x86-64 packages

AppImage, DEB, RPM and Arch/CachyOS packages share version **${version}**.
The Arch package is built from the same executable as the other formats.
The binary PKGBUILD and SHA256SUMS are attached.

Installation, CLI version and visible installer startup were checked on Ubuntu 22.04, Fedora 45 and Arch Linux.
See the [successful package checks](https://github.com/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}).

Office363 is experimental. These checks do not validate Microsoft 365 sign-in, document editing or OneNote synchronization.
EOF
options=()
if [[ "$GITHUB_REF_TYPE" == tag ]]; then
  options+=(--verify-tag)
fi
# gh stages the assets in a draft and publishes only after their uploads finish.
gh release create "v${version}" "$assets"/* \
  --repo "$GITHUB_REPOSITORY" --target "$GITHUB_SHA" \
  --title "Office363 ${version}" --generate-notes --notes-file "$notes" "${options[@]}"
