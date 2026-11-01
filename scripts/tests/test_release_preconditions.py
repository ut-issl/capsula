"""Tests for the release preconditions (requires Nushell 0.115.1 on PATH).

Run with: python3 -m unittest discover -s scripts/tests -p 'test_release_preconditions.py'
All Cargo metadata and crates.io responses are local fixtures.
"""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


SCRIPTS = Path(__file__).resolve().parents[1]
CRATES_API = "https://crates.io/api/v1/crates"


def package(name, publish=None, version="1.2.3"):
    return {"id": name, "name": name, "version": version, "publish": publish}


def metadata(packages, members=None):
    return {
        "packages": packages,
        "workspace_members": members if members is not None else [p["id"] for p in packages],
    }


class CratesServer:
    def __init__(self, responses):
        self.responses = responses
        self.requests = []
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                owner.requests.append((self.path, dict(self.headers)))
                status, body = owner.responses.get(self.path, (500, {"errors": []}))
                payload = json.dumps(body).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

            def log_message(self, format, *args):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *args):
        self.server.shutdown()
        self.thread.join()
        self.server.server_close()

    @property
    def url(self):
        return f"http://127.0.0.1:{self.server.server_port}/api/v1/crates"


class ReleasePreconditionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        cargo = bin_dir / "cargo"
        cargo.write_text(
            '#!/bin/sh\n'
            'if [ "$*" != "metadata --no-deps --format-version 1 --locked" ]; then\n'
            '  echo "unexpected cargo arguments: $*" >&2; exit 2\n'
            'fi\n'
            'if [ "${CAPSULA_TEST_CARGO_ERROR:-}" = 1 ]; then\n'
            '  echo "fixture cargo failure" >&2; exit 42\n'
            'fi\n'
            'printf "%s" "$CAPSULA_TEST_METADATA"\n'
        )
        cargo.chmod(0o755)
        self.env = os.environ.copy()
        self.env.update({
            "PATH": str(bin_dir) + os.pathsep + self.env.get("PATH", ""),
            "NO_COLOR": "1",
            "NO_PROXY": "127.0.0.1,localhost",
            "no_proxy": "127.0.0.1,localhost",
        })

    def run_script(self, name, cargo_metadata, *, server=None, cargo_error=False):
        script = SCRIPTS / name
        if server is not None:
            # The URL is a constant, so redirect a temporary copy to the local API.
            source = script.read_text()
            self.assertEqual(source.count(CRATES_API), 1)
            script = self.root / name
            script.write_text(source.replace(CRATES_API, server.url))
        env = self.env.copy()
        env["CAPSULA_TEST_METADATA"] = json.dumps(cargo_metadata)
        if cargo_error:
            env["CAPSULA_TEST_CARGO_ERROR"] = "1"
        return subprocess.run(
            ["nu", "--no-config-file", str(script)], cwd=self.root, env=env,
            text=True, capture_output=True, timeout=15, check=False,
        )

    def assert_success(self, result, message):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(message, result.stdout)

    def assert_failure(self, result, message):
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(message, result.stdout + result.stderr)

    def test_changelog_accepts_exact_and_dated_headings(self):
        cargo_metadata = metadata([package("other", version="9.9.9"), package("capsula")])
        for heading in ("## v1.2.3", "## v1.2.3 - 2026-01-02"):
            with self.subTest(heading=heading):
                (self.root / "CHANGELOG.md").write_text(f"# Changes\n{heading}\nNotes\n")
                result = self.run_script("check-changelog-release.nu", cargo_metadata)
                self.assert_success(result, "CHANGELOG.md has release notes for 1.2.3")

    def test_changelog_rejects_version_prefixes_and_wrong_headings(self):
        cargo_metadata = metadata([package("capsula")])
        for heading in ("## v1.2.30", "## v1.2.3-rc1", "## 1.2.3", "### v1.2.3", "## v1.2.3 extra"):
            with self.subTest(heading=heading):
                (self.root / "CHANGELOG.md").write_text(f"# Changes\n{heading}\n")
                result = self.run_script("check-changelog-release.nu", cargo_metadata)
                self.assert_failure(result, "has no release notes for 1.2.3")

    def test_changelog_reports_missing_file(self):
        result = self.run_script("check-changelog-release.nu", metadata([package("capsula")]))
        self.assert_failure(result, "CHANGELOG.md does not exist")

    def test_both_scripts_report_cargo_metadata_failure(self):
        for name in ("check-changelog-release.nu", "check-crates-io-release.nu"):
            with self.subTest(script=name):
                result = self.run_script(name, {}, cargo_error=True)
                self.assert_failure(result, "failed to read Cargo workspace metadata")
                self.assertIn("fixture cargo failure", result.stdout + result.stderr)

    def test_crates_selects_only_publishable_workspace_members_in_name_order(self):
        packages = [
            package("z-default"), package("private", []), package("elsewhere", ["other"]),
            package("a-allowed", ["other", "crates-io"], version="2.0.0"),
            package("dependency"),
        ]
        responses = {
            "/api/v1/crates/a-allowed": (200, {"crate": {"trustpub_only": True}}),
            "/api/v1/crates/z-default": (200, {"crate": {"trustpub_only": True}}),
        }
        with CratesServer(responses) as server:
            result = self.run_script(
                "check-crates-io-release.nu",
                metadata(packages, ["z-default", "private", "elsewhere", "a-allowed"]),
                server=server,
            )
            self.assert_success(result, "All release crates exist on crates.io and require Trusted Publishing")
            self.assertEqual([path for path, _ in server.requests], list(responses))
            for _, headers in server.requests:
                self.assertEqual(headers.get("Accept"), "application/json")
                self.assertIn("capsula-release-precondition", headers.get("User-Agent", ""))
            self.assertIn("2.0.0", result.stdout)

    def test_crates_rejects_empty_selection_without_network_request(self):
        with CratesServer({}) as server:
            result = self.run_script(
                "check-crates-io-release.nu",
                metadata([package("private", []), package("elsewhere", ["other"])]),
                server=server,
            )
            self.assert_failure(result, "would not select any crates for crates.io")
            self.assertEqual(server.requests, [])

    def test_crates_rejects_missing_crate(self):
        with CratesServer({"/api/v1/crates/missing": (404, {"errors": []})}) as server:
            result = self.run_script("check-crates-io-release.nu", metadata([package("missing")]), server=server)
            self.assert_failure(result, "do not exist on crates.io")
            self.assertIn("missing", result.stdout + result.stderr)

    def test_crates_reports_missing_before_trusted_publishing(self):
        responses = {
            "/api/v1/crates/missing": (404, {"errors": []}),
            "/api/v1/crates/unprotected": (200, {"crate": {"trustpub_only": False}}),
        }
        with CratesServer(responses) as server:
            result = self.run_script(
                "check-crates-io-release.nu",
                metadata([package("unprotected"), package("missing")]), server=server,
            )
            self.assert_failure(result, "do not exist on crates.io")
            self.assertNotIn("do not require Trusted Publishing", result.stdout + result.stderr)
            self.assertEqual([path for path, _ in server.requests], list(responses))

    def test_crates_rejects_disabled_or_absent_trusted_publishing(self):
        for crate_info in ({"trustpub_only": False}, {}):
            with self.subTest(crate_info=crate_info), CratesServer({
                "/api/v1/crates/example": (200, {"crate": crate_info})
            }) as server:
                result = self.run_script("check-crates-io-release.nu", metadata([package("example")]), server=server)
                self.assert_failure(result, "do not require Trusted Publishing")
                self.assertIn("example", result.stdout + result.stderr)

    def test_crates_reports_unexpected_http_status_and_details(self):
        with CratesServer({
            "/api/v1/crates/example": (503, {"errors": [{"detail": "try again later"}]})
        }) as server:
            result = self.run_script("check-crates-io-release.nu", metadata([package("example")]), server=server)
            self.assert_failure(result, "crates.io returned HTTP 503 for `example`")
            self.assertIn("try again later", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
