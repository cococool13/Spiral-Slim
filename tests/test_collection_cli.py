import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from contextlib import redirect_stdout
from unittest.mock import patch

from browser_collection.models import (
    BrowserInstallation,
    BrowserPlan,
    PlannedControl,
    PreviewResult,
    ResolvedProfile,
    Risk,
    SupportState,
)


ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "browser_collection.py"


def load_cli_module():
    spec = importlib.util.spec_from_file_location(
        "browser_collection_cli",
        CLI,
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def example_preview(*, blocked=False):
    profile = ResolvedProfile(
        id="balanced-daily",
        name="Balanced Daily",
        description="Example",
        risk=Risk.LOW,
        modules=("security-foundation",),
        controls=(),
    )
    installation = BrowserInstallation(
        browser_id="brave",
        name="Brave",
        platform="macos",
        path="/Applications/Brave Browser.app",
    )
    control = PlannedControl(
        control_id="security.safe-browsing",
        vendor_name="SafeBrowsingProtectionLevel",
        current_value=None,
        desired_value=1,
        action="unsupported" if blocked else "add",
        support=(
            SupportState.UNSUPPORTED
            if blocked
            else SupportState.PREVIEW_READY
        ),
        required=True,
        reason="No verified mapping." if blocked else "",
    )
    return PreviewResult(
        schema_version=1,
        profile=profile,
        browser_plans=(
            BrowserPlan("brave", installation, (control,)),
        ),
        plan_hash="a" * 64,
        blocked=blocked,
    )


class CliTests(unittest.TestCase):
    def run_cli(self, *args):
        return subprocess.run(
            [sys.executable, str(CLI), *args],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )

    def test_catalog_json_is_valid_without_elevation(self):
        result = self.run_cli("--catalog", "--format", "json")
        payload = json.loads(result.stdout)
        self.assertEqual(0, result.returncode)
        self.assertEqual("spiral-browser-collection", payload["tool"]["id"])
        self.assertFalse(payload["tool"]["mutating_commands_available"])
        self.assertEqual("", result.stderr)

    def test_catalog_json_is_deterministic(self):
        first = self.run_cli("--catalog", "--format", "json")
        second = self.run_cli("--catalog", "--format", "json")
        self.assertEqual(0, first.returncode)
        self.assertEqual(first.stdout, second.stdout)

    def test_detect_json_has_explicit_installation_fields(self):
        result = self.run_cli(
            "--detect",
            "--browser",
            "brave",
            "--format",
            "json",
        )
        payload = json.loads(result.stdout)
        self.assertEqual(0, result.returncode)
        self.assertEqual(["brave"], sorted(payload))
        for installation in payload["brave"]:
            self.assertEqual(
                {
                    "browser_id",
                    "name",
                    "path",
                    "platform",
                    "version",
                },
                set(installation),
            )

    def test_preview_json_declares_read_only(self):
        result = self.run_cli(
            "--preview",
            "balanced-daily",
            "--browser",
            "brave",
            "--format",
            "json",
        )
        payload = json.loads(result.stdout)
        self.assertEqual(0, result.returncode)
        self.assertFalse(payload["mutates_system"])
        self.assertEqual("preview", payload["operation"])
        self.assertEqual(64, len(payload["plan_hash"]))

    def test_apply_flag_does_not_exist_in_milestone_one(self):
        result = self.run_cli("--apply", "balanced-daily")
        self.assertEqual(2, result.returncode)
        self.assertIn("unrecognized arguments", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_actions_are_mutually_exclusive(self):
        result = self.run_cli("--catalog", "--detect")
        self.assertEqual(2, result.returncode)
        self.assertIn("not allowed with argument", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_catalog_rejects_browser_selection(self):
        result = self.run_cli("--catalog", "--browser", "brave")
        self.assertEqual(2, result.returncode)
        self.assertIn("--browser is only valid", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_unknown_browser_is_a_clean_configuration_error(self):
        result = self.run_cli("--detect", "--browser", "unknown")
        self.assertEqual(2, result.returncode)
        self.assertIn("unknown browser", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_unknown_profile_is_a_clean_configuration_error(self):
        result = self.run_cli("--preview", "missing", "--browser", "brave")
        self.assertEqual(2, result.returncode)
        self.assertIn("unknown profile", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_empty_browser_selection_is_rejected(self):
        result = self.run_cli("--detect", "--browser", ",")
        self.assertEqual(2, result.returncode)
        self.assertIn("at least one browser", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_blocked_preview_returns_exit_code_three(self):
        cli = load_cli_module()

        class BlockedEngine:
            def preview(self, profile_id, browser_ids):
                self.profile_id = profile_id
                self.browser_ids = browser_ids
                return example_preview(blocked=True)

        output = io.StringIO()
        # Nested rather than parenthesized: the repo's ruff target is py38,
        # where `with (a, b):` is a syntax error.
        with patch.object(cli, "build_engine", return_value=BlockedEngine()), \
                redirect_stdout(output):
            result = cli.main([
                "--preview",
                "balanced-daily",
                "--browser",
                "brave",
                "--format",
                "json",
            ])
        payload = json.loads(output.getvalue())
        self.assertEqual(3, result)
        self.assertTrue(payload["blocked"])


class RenderTests(unittest.TestCase):
    def test_preview_json_renderer_is_deterministic_and_complete(self):
        from browser_collection.render import preview_to_dict

        first = preview_to_dict(example_preview())
        second = preview_to_dict(example_preview())
        self.assertEqual(first, second)
        self.assertEqual("brave", first["browsers"][0]["id"])
        self.assertEqual(
            "SafeBrowsingProtectionLevel",
            first["browsers"][0]["controls"][0]["vendor_name"],
        )

    def test_preview_text_renderer_has_stable_summary(self):
        from browser_collection.render import render_preview_text

        self.assertEqual(
            "\n".join((
                "Preview only — no changes will be made.",
                "Profile: Balanced Daily (low)",
                f"Plan: {'a' * 64}",
                "Brave: /Applications/Brave Browser.app",
                "  1 add",
            )),
            render_preview_text(example_preview()),
        )

    def test_blocked_preview_text_explains_block(self):
        from browser_collection.render import render_preview_text

        rendered = render_preview_text(example_preview(blocked=True))
        self.assertIn(
            "Blocked: at least one required control is unsupported.",
            rendered,
        )


if __name__ == "__main__":
    unittest.main()
