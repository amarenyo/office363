import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("prepare_release", Path(__file__).resolve().parents[1] / "ci/prepare-release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "package.json").write_text(json.dumps({"version": "1.2.3"}))
        self.artifacts = self.root / "artifacts"
        self.output = self.root / "release"
        self.packages = []
        for folder, name in {
            "appimage": "Office363_1.2.3_amd64.AppImage",
            "deb": "Office363_1.2.3_amd64.deb",
            "rpm": "Office363-1.2.3-1.x86_64.rpm",
            "arch-cachyos": "office363-1.2.3-1-x86_64.pkg.tar.zst",
        }.items():
            directory = self.artifacts / f"{folder}-x86_64"
            directory.mkdir(parents=True)
            package = directory / name
            package.write_bytes(name.encode())
            (directory / "SHA256SUMS").write_text(f"{release.digest(package)}  ./{name}\n")
            self.packages.append(package)
        (self.artifacts / "arch-cachyos-x86_64/PKGBUILD").write_text("pkgver=1.2.3\npkgrel=1\n")

    def test_collects_four_packages_recipe_and_unified_checksums(self):
        self.assertEqual(release.prepare(self.root, self.artifacts, self.output), "1.2.3")
        self.assertEqual(len(list(self.output.iterdir())), 6)
        lines = (self.output / "SHA256SUMS").read_text().splitlines()
        self.assertEqual(len(lines), 5)
        for line in lines:
            checksum, name = line.split(maxsplit=1)
            self.assertEqual(release.digest(self.output / name), checksum)

    def test_rejects_corrupt_package_without_partial_release(self):
        self.packages[0].write_bytes(b"corrupted")
        with self.assertRaisesRegex(ValueError, "checksum"):
            release.prepare(self.root, self.artifacts, self.output)
        self.assertFalse(self.output.exists())

    def test_rejects_missing_format(self):
        self.packages[-1].unlink()
        with self.assertRaises(FileNotFoundError):
            release.prepare(self.root, self.artifacts, self.output)
        self.assertFalse(self.output.exists())

    def test_rejects_recipe_from_a_different_version(self):
        (self.artifacts / "arch-cachyos-x86_64/PKGBUILD").write_text("pkgver=9.0.0\n")
        with self.assertRaisesRegex(ValueError, "version"):
            release.prepare(self.root, self.artifacts, self.output)
        self.assertFalse(self.output.exists())

    def test_refuses_to_mix_with_existing_assets(self):
        self.output.mkdir()
        with self.assertRaises(FileExistsError):
            release.prepare(self.root, self.artifacts, self.output)
