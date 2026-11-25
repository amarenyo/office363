#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
input=$(realpath "${1:?Usage: prepare-arch-binary.sh ARTIFACT_DIRECTORY OUTPUT_DIRECTORY}")
output=$(realpath -m "${2:?Missing output directory}")
version=$(python3 -c 'import json; print(json.load(open("package.json"))["version"])')
(cd "$input" && sha256sum --check --ignore-missing SHA256SUMS)
mkdir -p "$output"
cp "$input/Office363_${version}_amd64.deb" "$output/office363-${version}-amd64.deb"
cp "$root/LICENSE" "$output/LICENSE"
deb_sha=$(sha256sum "$output/office363-${version}-amd64.deb" | cut -d' ' -f1)
license_sha=$(sha256sum "$output/LICENSE" | cut -d' ' -f1)
sed -e "s/@VERSION@/${version}/g" -e "s/@DEB_SHA256@/${deb_sha}/g" -e "s/@LICENSE_SHA256@/${license_sha}/g" \
  "$root/packaging/arch/PKGBUILD-bin.in" > "$output/PKGBUILD"
