import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("release_gate", Path(__file__).resolve().parents[1] / "ci/check-release.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class ReleaseGateTests(unittest.TestCase):
    def test_allows_new_main_release_and_existing_matching_tag(self):
        with patch.object(gate, "optional_api", side_effect=[None, None]):
            gate.check_release("owner/repo", "refs/heads/main", "abc", "1.2.3")
        with patch.object(gate, "optional_api", side_effect=[None, {"ref": "tag"}, {"sha": "abc"}]):
            gate.check_release("owner/repo", "refs/tags/v1.2.3", "abc", "1.2.3")

    def test_refuses_other_branches_and_mismatched_tags(self):
        for ref in ["refs/heads/feature", "refs/tags/v9.0.0"]:
            with self.assertRaisesRegex(RuntimeError, "only from main"):
                gate.check_release("owner/repo", ref, "abc", "1.2.3")

    def test_refuses_existing_release(self):
        with patch.object(gate, "optional_api", return_value={"id": 1}):
            with self.assertRaisesRegex(RuntimeError, "already exists"):
                gate.check_release("owner/repo", "refs/heads/main", "abc", "1.2.3")

    def test_refuses_tag_pointing_at_different_commit(self):
        with patch.object(gate, "optional_api", side_effect=[None, {"ref": "tag"}, {"sha": "other"}]):
            with self.assertRaisesRegex(RuntimeError, "different commit"):
                gate.check_release("owner/repo", "refs/heads/main", "abc", "1.2.3")

    def test_only_404_means_absent_not_auth_or_network_failure(self):
        with patch.object(gate.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "", "gh: Not Found (HTTP 404)")):
            self.assertIsNone(gate.optional_api("repos/owner/repo/releases/tags/v1.2.3"))
        for error in ["gh: Forbidden (HTTP 403)", "network timeout"]:
            with patch.object(gate.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "", error)):
                with self.assertRaises(RuntimeError):
                    gate.optional_api("repos/owner/repo/releases/tags/v1.2.3")
