#!/usr/bin/env python3
"""Fail before spending build minutes on an existing release or conflicting tag."""
import json
import os
from pathlib import Path
import subprocess


def optional_api(endpoint):
    result = subprocess.run(["gh", "api", endpoint], capture_output=True, text=True)
    if result.returncode == 0:
        return json.loads(result.stdout)
    if "(HTTP 404)" in result.stderr:
        return None
    raise RuntimeError(result.stderr)


def check_release(repo, ref, sha, version):
    tag = f"v{version}"
    if ref not in ("refs/heads/main", f"refs/tags/{tag}"):
        raise RuntimeError("Publish releases only from main or the matching version tag.")
    if optional_api(f"repos/{repo}/releases/tags/{tag}") is not None:
        raise RuntimeError(f"Release {tag} already exists. Bump the version before publishing again.")
    if optional_api(f"repos/{repo}/git/ref/tags/{tag}") is not None:
        # The commits endpoint resolves both lightweight and annotated tags.
        commit = optional_api(f"repos/{repo}/commits/{tag}")
        if commit is None or commit["sha"] != sha:
            raise RuntimeError(f"Tag {tag} already points to a different commit. Bump the version.")


if __name__ == "__main__":
    version = json.loads((Path(__file__).resolve().parents[2] / "package.json").read_text())["version"]
    check_release(os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_REF"], os.environ["GITHUB_SHA"], version)
