#!/usr/bin/env python3
"""Export an issue and every paginated comment to offline HTML, Markdown and JSON."""

import argparse
import html
import json
import os
import urllib.error
import urllib.request
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def github_json(endpoint, *, paginate=False):
    command = ["gh", "api", "--hostname", "github.com"]
    if paginate:
        command += ["--paginate", "--slurp"]
    command += [endpoint]
    try:
        result = subprocess.run(
            command, check=True, capture_output=True, text=True, timeout=120
        )
    except FileNotFoundError:
        return github_http_json(endpoint, paginate=paginate)
    return json.loads(result.stdout)


def github_http_json(endpoint, *, paginate=False):
    """Use the public REST API when GitHub CLI is unavailable."""
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "mockforge-issue-export",
    }
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    pages = []
    page = 1
    while True:
        suffix = f"&page={page}" if paginate else ""
        request = urllib.request.Request(
            f"https://api.github.com/{endpoint}{suffix}", headers=headers
        )
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                data = json.load(response)
        except urllib.error.HTTPError as error:
            raise ValueError(
                f"GitHub API returned HTTP {error.code}. For rate limits or private repositories, "
                "set GH_TOKEN or GITHUB_TOKEN, or install gh and run gh auth login."
            ) from None
        if not paginate:
            return data
        if not isinstance(data, list):
            raise ValueError("GitHub comments response must be a list")
        pages.append(data)
        if len(data) < 100:
            return pages
        page += 1


def fetch_snapshot(repository, number):
    """Read only from the explicitly selected repository on github.com."""
    endpoint = f"repos/{repository}/issues/{number}"
    issue = github_json(endpoint)
    pages = github_json(f"{endpoint}/comments?per_page=100", paginate=True)
    comments = [comment for page in pages for comment in page]
    return {
        "exported_at": datetime.now(timezone.utc).isoformat(),
        "repository": repository,
        "issue": issue,
        "comments": comments,
    }


def validate_snapshot(snapshot, repository, number):
    issue = snapshot["issue"]
    expected = f"https://github.com/{repository}/issues/{number}"
    if snapshot["repository"] != repository or issue["html_url"] != expected:
        raise ValueError("Snapshot does not match the selected repository/issue")
    if issue["number"] != number or "pull_request" in issue:
        raise ValueError("Expected an issue, not a pull request")
    comments = snapshot["comments"]
    if len(comments) != issue["comments"]:
        raise ValueError("Comment count changed or export is incomplete; fetch again")
    ids = [comment["id"] for comment in comments]
    if len(set(ids)) != len(ids):
        raise ValueError("Duplicate comment IDs; fetch again")
    for comment in comments:
        if comment["html_url"] != f"{expected}#issuecomment-{comment['id']}":
            raise ValueError("Comment URL does not belong to the selected issue")
    return sorted(comments, key=lambda c: (c["created_at"], c["id"]))


def author(item):
    return (item.get("user") or {}).get("login") or "[deleted account]"


def render(snapshot, comments):
    issue = snapshot["issue"]
    title = f"{snapshot['repository']} #{issue['number']}: {issue['title']}"
    note = (
        f"Exported {snapshot['exported_at']}. {len(comments)} comments. "
        f"Issue state: {issue['state']}. "
        "Snapshot of the issue body and all available issue comments. "
        "GitHub edits after export, deleted comments, timeline events, reactions, "
        "linked PR discussions and attachment contents are not included. "
        "Attachment URLs remain in the text; no external content is downloaded. "
        "Fetching multiple pages is not an atomic snapshot."
    )
    escape = html.escape
    html_parts = [
        '<!doctype html><html lang="en"><head><meta charset="utf-8">',
        '<meta name="viewport" content="width=device-width, initial-scale=1">',
        '<meta http-equiv="Content-Security-Policy" '
        "content=\"default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'\">",
        f"<title>{escape(title)}</title>",
        "<style>body{max-width:72rem;margin:2rem auto;padding:0 1rem;"
        "font:16px/1.5 system-ui;color:#17202a;background:#fff}"
        "pre{white-space:pre-wrap;overflow-wrap:anywhere;font:14px/1.5 monospace}"
        "article{border-top:1px solid #bbb;padding:1rem 0}"
        "a{color:#174ea6}nav{line-height:2}"
        "@media print{body{max-width:none;margin:0}nav{display:none}"
        "h2{break-after:avoid}pre{font-size:10pt}}</style></head><body>",
        f"<h1>{escape(title)}</h1><p>{escape(note)}</p>",
        "<p>Search with Ctrl+F (Cmd+F on macOS). Print and choose Save as PDF "
        "for a searchable PDF. Bodies are shown as plain Markdown source so "
        'code, logs and tables remain complete.</p><nav><a href="#issue">Issue</a> ',
    ]
    html_parts.extend(
        f'<a href="#comment-{c["id"]}">{i}</a> ' for i, c in enumerate(comments, 1)
    )
    html_parts.append("</nav>")
    markdown = [f"# {escape(title)}\n\n{escape(note)}\n\nSource: {issue['html_url']}\n"]
    for index, item in enumerate([issue, *comments]):
        heading = "Issue" if index == 0 else f"Comment {index}"
        anchor = "issue" if index == 0 else f"comment-{item['id']}"
        metadata = (
            f"{author(item)} | created {item['created_at']} | "
            f"updated {item['updated_at']}"
        )
        body = item.get("body") or ""
        html_parts.append(
            f'<article id="{anchor}"><h2>{heading}</h2>'
            f"<p>{escape(metadata)} | "
            f'<a href="{escape(item["html_url"], quote=True)}">View on GitHub</a></p>'
            f"<pre>{escape(body)}</pre></article>"
        )
        # Use a longer fence than anything in a body. Raw Markdown is retained
        # as text, including HTML which a Markdown viewer might otherwise run.
        runs = re.findall(r"`+", body)
        fence = "`" * max(3, 1 + max(map(len, runs), default=0))
        markdown.append(
            f"\n## {heading}\n\n{escape(metadata)}\n\n{item['html_url']}\n\n"
            f"{fence}text\n{body}\n{fence}\n"
        )
    html_parts.append("</body></html>\n")
    return "\n".join(html_parts), "".join(markdown)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default="SaaSy-Solutions/mockforge")
    parser.add_argument("--issue", type=int, default=79)
    parser.add_argument(
        "--output-dir", type=Path, required=True, help="New output directory"
    )
    parser.add_argument(
        "--input", type=Path, help="Offline snapshot.json from a previous export"
    )
    args = parser.parse_args(argv)
    if (
        not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repo)
        or args.issue < 1
    ):
        parser.error("Expected --repo OWNER/REPO and a positive --issue number")
    try:
        snapshot = (
            json.loads(args.input.read_text(encoding="utf-8"))
            if args.input
            else fetch_snapshot(args.repo, args.issue)
        )
        comments = validate_snapshot(snapshot, args.repo, args.issue)
        html_text, markdown = render(snapshot, comments)
        # Never overwrite an earlier snapshot or leave an output on fetch failure.
        args.output_dir.mkdir(parents=True, exist_ok=False)
        (args.output_dir / "discussion.html").write_text(html_text, encoding="utf-8")
        (args.output_dir / "discussion.md").write_text(markdown, encoding="utf-8")
        (args.output_dir / "snapshot.json").write_text(
            json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
    except subprocess.TimeoutExpired:
        print(
            "GitHub read timed out; no export created. Try again later.",
            file=sys.stderr,
        )
        return 1
    except subprocess.CalledProcessError as error:
        print(
            f"GitHub read failed (exit {error.returncode}); no export created. Check gh auth status.",
            file=sys.stderr,
        )
        return 1
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Export failed: {error}", file=sys.stderr)
        return 1
    print(
        f"Exported issue + {len(comments)} comments to {args.output_dir / 'discussion.html'}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
