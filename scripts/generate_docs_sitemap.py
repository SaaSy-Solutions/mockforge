#!/usr/bin/env python3
"""
Write sitemap.xml for the built mdBook docs (docs.mockforge.dev).

mdBook 0.4 has no sitemap output, so the deploy-docs workflow runs this
after `mdbook build`. Only pages rendered from a chapter in book/src are
listed; 404.html, print.html, toc.html and copied static HTML are skipped.

Usage: generate_docs_sitemap.py [--book-dir book] [--base-url URL]
"""

import argparse
import sys
from pathlib import Path
from xml.sax.saxutils import escape

DEFAULT_BASE_URL = "https://docs.mockforge.dev/"


def page_urls(src_dir: Path, out_dir: Path, base_url: str) -> list[str]:
    return [url for _, url in chapter_pages(src_dir, out_dir, base_url)]


def chapter_pages(src_dir: Path, out_dir: Path, base_url: str) -> list[tuple[Path, str]]:
    """(built HTML file, public URL) for every page rendered from a chapter."""
    pages = []
    for html in sorted(out_dir.rglob("*.html")):
        rel = html.relative_to(out_dir)
        sources = [src_dir / rel.with_suffix(".md")]
        if rel.name == "index.html":
            # mdBook renders a chapter's README.md as index.html.
            sources.append(src_dir / rel.parent / "README.md")
        if not any(source.is_file() for source in sources):
            continue
        path = rel.as_posix()
        if rel.name == "index.html":
            path = path[: -len("index.html")]
        pages.append((html, base_url + path))
    return pages


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1])
    parser.add_argument("--book-dir", default="book", type=Path)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    args = parser.parse_args()

    base_url = args.base_url.rstrip("/") + "/"
    src_dir = args.book_dir / "src"
    out_dir = args.book_dir / "book"
    if not out_dir.is_dir():
        print(f"error: {out_dir} not found; run `mdbook build` first", file=sys.stderr)
        return 1

    urls = page_urls(src_dir, out_dir, base_url)
    if not urls:
        print("error: no chapter pages found in build output", file=sys.stderr)
        return 1

    lines = ['<?xml version="1.0" encoding="UTF-8"?>']
    lines.append('<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">')
    lines.extend(f"  <url><loc>{escape(url)}</loc></url>" for url in urls)
    lines.append("</urlset>")
    (out_dir / "sitemap.xml").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {out_dir / 'sitemap.xml'} with {len(urls)} URLs")
    return 0


if __name__ == "__main__":
    sys.exit(main())
