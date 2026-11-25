#!/usr/bin/env python3
"""Validate all tested artifacts before assembling the public release assets."""
import hashlib
import json
from pathlib import Path
import shutil
import sys


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def prepare(root, artifacts, output):
    version = json.loads((root / "package.json").read_text())["version"]
    packages = {
        "appimage-x86_64": f"Office363_{version}_amd64.AppImage",
        "deb-x86_64": f"Office363_{version}_amd64.deb",
        "rpm-x86_64": f"Office363-{version}-1.x86_64.rpm",
        "arch-cachyos-x86_64": f"office363-{version}-1-x86_64.pkg.tar.zst",
    }
    files = []
    for artifact, filename in packages.items():
        folder = artifacts / artifact
        checksums = dict(
            (name.lstrip("*"), checksum)
            for checksum, name in (
                line.split(maxsplit=1)
                for line in (folder / "SHA256SUMS").read_text().splitlines()
            )
        )
        path = folder / filename
        expected = checksums.get(filename, checksums.get(f"./{filename}"))
        if expected is None or digest(path) != expected:
            raise ValueError(f"Missing or incorrect checksum: {path}")
        files.append(path)
    recipe = artifacts / "arch-cachyos-x86_64" / "PKGBUILD"
    if f"pkgver={version}\n" not in recipe.read_text():
        raise ValueError("Arch recipe version differs from package.json")
    files.append(recipe)
    # Refuse to mix assets from different runs or leave partial output on validation failure.
    output.mkdir(parents=True, exist_ok=False)
    for path in files:
        shutil.copyfile(path, output / path.name)
    (output / "SHA256SUMS").write_text("".join(
        f"{digest(path)}  {path.name}\n" for path in sorted(output.iterdir())
    ))
    return version


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("Usage: prepare-release.py ARTIFACT_DIRECTORY OUTPUT_DIRECTORY")
    prepare(Path(__file__).resolve().parents[2], Path(sys.argv[1]), Path(sys.argv[2]))
