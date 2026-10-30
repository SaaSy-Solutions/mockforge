"""Offline regressions for pagination, provenance and untrusted comment rendering."""

import copy
import importlib.util
import json
import subprocess
import tempfile
import unittest
from html.parser import HTMLParser
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "export_github_issue", ROOT / "scripts/export_github_issue.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
URL = "https://github.com/SaaSy-Solutions/mockforge/issues/79"


def snapshot(count=2):
    issue = {
        "html_url": URL,
        "number": 79,
        "title": "A searchable discussion",
        "comments": count,
        "state": "closed",
        "body": "Original report",
        "user": {"login": "reporter"},
        "created_at": "2025-11-25T05:20:09Z",
        "updated_at": "2026-10-24T01:19:07Z",
    }
    comments = [
        dict(issue, id=i, html_url=f"{URL}#issuecomment-{i}", body=f"Body {i}")
        for i in range(1, count + 1)
    ]
    return {
        "repository": "SaaSy-Solutions/mockforge",
        "exported_at": "2026-10-24T15:00:00Z",
        "issue": issue,
        "comments": comments,
    }


class Tags(HTMLParser):
    def __init__(self):
        super().__init__()
        self.tags = []
        self.text = []

    def handle_starttag(self, tag, attrs):
        self.tags.append(tag)

    def handle_data(self, data):
        self.text.append(data)


class ExportTests(unittest.TestCase):
    def test_fetch_requests_all_pages_and_explicit_github_host(self):
        data = snapshot(205)
        pages = [
            data["comments"][:100],
            data["comments"][100:200],
            data["comments"][200:],
        ]
        results = [
            subprocess.CompletedProcess([], 0, json.dumps(value), "")
            for value in (data["issue"], pages)
        ]
        with patch.object(MODULE.subprocess, "run", side_effect=results) as run:
            fetched = MODULE.fetch_snapshot(data["repository"], 79)
        self.assertEqual(
            len(MODULE.validate_snapshot(fetched, data["repository"], 79)), 205
        )
        command = run.call_args_list[1].args[0]
        self.assertEqual(
            command,
            [
                "gh",
                "api",
                "--hostname",
                "github.com",
                "--paginate",
                "--slurp",
                "repos/SaaSy-Solutions/mockforge/issues/79/comments?per_page=100",
            ],
        )
        self.assertNotIn("shell", run.call_args.kwargs)

    def test_all_bodies_are_preserved_and_html_never_runs_comment_markup(self):
        data = snapshot()
        body = '<script>alert(1)</script><img src="https://invalid.example/x">\n```js\n`a`\n```\nSrikanth: café 漢字'
        data["comments"][0]["body"] = body
        data["comments"][0]["user"] = {"login": '<img src=x onerror="alert(2)">'}
        html_text, markdown = MODULE.render(data, data["comments"])
        parser = Tags()
        parser.feed(html_text)
        self.assertNotIn("script", parser.tags)
        self.assertNotIn("img", parser.tags)
        self.assertIn(body, "".join(parser.text))
        self.assertIn("````text\n" + body + "\n````", markdown)
        self.assertIn("Body 2", markdown)
        self.assertIn("default-src 'none'", html_text)

    def test_markdown_metadata_cannot_embed_html(self):
        data = snapshot()
        data["issue"]["title"] = "<script>title</script>"
        data["exported_at"] = "<img src=x>"
        data["comments"][0]["user"] = {"login": "<script>author</script>"}
        _, markdown = MODULE.render(data, data["comments"])
        self.assertNotIn("<script>", markdown)
        self.assertNotIn("<img", markdown)
        self.assertIn("&lt;script&gt;title&lt;/script&gt;", markdown)

    def test_timeout_does_not_leave_an_export(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "absent"
            with patch.object(
                MODULE,
                "fetch_snapshot",
                side_effect=subprocess.TimeoutExpired("gh", 120),
            ):
                self.assertEqual(MODULE.main(["--output-dir", str(output)]), 1)
            self.assertFalse(output.exists())

    def test_missing_page_is_rejected(self):
        data = snapshot()
        data["comments"].pop()
        with self.assertRaisesRegex(ValueError, "incomplete"):
            MODULE.validate_snapshot(data, data["repository"], 79)

    def test_duplicate_ids_are_rejected(self):
        data = snapshot()
        data["comments"][1] = copy.deepcopy(data["comments"][0])
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            MODULE.validate_snapshot(data, data["repository"], 79)

    def test_wrong_issue_or_pr_is_rejected(self):
        for mutate in (
            lambda d: d.update(repository="someone/else"),
            lambda d: d["issue"].update(html_url=URL + "0"),
            lambda d: d["issue"].update(number=80),
            lambda d: d["issue"].update(pull_request={}),
            lambda d: d["comments"][0].update(html_url="javascript:alert(1)"),
        ):
            data = snapshot()
            mutate(data)
            with self.assertRaises(ValueError):
                MODULE.validate_snapshot(data, "SaaSy-Solutions/mockforge", 79)

    def test_deleted_user_empty_body_and_zero_comments(self):
        data = snapshot(0)
        data["issue"].update(user=None, body=None)
        comments = MODULE.validate_snapshot(data, data["repository"], 79)
        html_text, markdown = MODULE.render(data, comments)
        self.assertIn("[deleted account]", html_text)
        self.assertIn("0 comments", markdown)

    def test_chronological_order_has_stable_ids(self):
        data = snapshot()
        data["comments"].reverse()
        self.assertEqual(
            [c["id"] for c in MODULE.validate_snapshot(data, data["repository"], 79)],
            [1, 2],
        )

    def test_offline_cli_writes_all_three_files_without_network(self):
        data = snapshot()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "input.json"
            source.write_text(json.dumps(data), encoding="utf-8")
            output = root / "output"
            with patch.object(
                MODULE.subprocess, "run", side_effect=AssertionError("Network used")
            ):
                self.assertEqual(
                    MODULE.main(["--input", str(source), "--output-dir", str(output)]),
                    0,
                )
            self.assertEqual(len(list(output.iterdir())), 3)
            self.assertEqual(json.loads((output / "snapshot.json").read_text()), data)
            original = (output / "discussion.html").read_bytes()
            self.assertEqual(
                MODULE.main(["--input", str(source), "--output-dir", str(output)]), 1
            )
            self.assertEqual((output / "discussion.html").read_bytes(), original)

    def test_auth_failure_does_not_leave_export_or_disclose_stderr(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "absent"
            error = subprocess.CalledProcessError(
                1, ["gh"], stderr="sensitive diagnostic"
            )
            with (
                patch.object(MODULE, "fetch_snapshot", side_effect=error),
                patch("sys.stderr") as stderr,
            ):
                self.assertEqual(MODULE.main(["--output-dir", str(output)]), 1)
            self.assertFalse(output.exists())
            self.assertNotIn("sensitive diagnostic", str(stderr.write.call_args_list))

    def test_invalid_repo_never_reaches_gh(self):
        with patch.object(MODULE, "fetch_snapshot") as fetch:
            with self.assertRaises(SystemExit):
                MODULE.main(["--repo", "../else/issues", "--output-dir", "unused"])
            fetch.assert_not_called()


if __name__ == "__main__":
    unittest.main()
