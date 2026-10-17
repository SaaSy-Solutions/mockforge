#!/usr/bin/env python3
"""
Add <link rel="canonical"> to every chapter page of the built mdBook docs.

mdBook 0.4 emits no canonical tag, and its theme templates cannot build one:
the only page-path variable is the source `path` (e.g. `guide/intro.md`) and
mdBook registers no string helpers to turn that into the output `.html` URL.
So the deploy-docs workflow runs this after `mdbook build`. URLs come from the
same chapter mapping as generate_docs_sitemap.py, so canonicals and sitemap
always agree (README.md chapters canonicalize to their directory URL).

Idempotent: an existing canonical link is replaced.

Usage: inject_docs_canonicals.py [--book-dir book] [--base-url URL]
"""

import argparse
import html
import re
import sys
from pathlib import Path

from generate_docs_sitemap import DEFAULT_BASE_URL, chapter_pages

CANONICAL_RE = re.compile(r'\s*<link rel="canonical"[^>]*>')


def inject(page: str, url: str) -> str:
    page = CANONICAL_RE.sub("", page)
    tag = f'<link rel="canonical" href="{html.escape(url, quote=True)}">'
    head_close = page.find("</head>")
    if head_close == -1:
        raise ValueError("no </head> in page")
    return page[:head_close] + "    " + tag + "\n    " + page[head_close:]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1])
    parser.add_argument("--book-dir", default="book", type=Path)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    args = parser.parse_args()

    base_url = args.base_url.rstrip("/") + "/"
    out_dir = args.book_dir / "book"
    if not out_dir.is_dir():
        print(f"error: {out_dir} not found; run `mdbook build` first", file=sys.stderr)
        return 1

    pages = chapter_pages(args.book_dir / "src", out_dir, base_url)
    if not pages:
        print("error: no chapter pages found in build output", file=sys.stderr)
        return 1

    for path, url in pages:
        path.write_text(inject(path.read_text(encoding="utf-8"), url), encoding="utf-8")
    print(f"added canonical links to {len(pages)} pages")
    return 0


if __name__ == "__main__":
    sys.exit(main())
