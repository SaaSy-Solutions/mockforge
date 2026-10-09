"""Classify plugin publication requests and constrain their checkout-local path.

This only establishes request intent and a manifest location. It does not
validate package contents, log in, build, or publish anything.
"""

from pathlib import Path, PurePosixPath
import os
import re
import sys


def plugin_directory(workspace, event_name, ref, requested_path):
    """Return a safe relative directory, or None for a non-plugin push."""
    if event_name == "push":
        prefix = "refs/tags/plugin-"
        if not ref.startswith(prefix):
            return None
        if ref == prefix:
            raise ValueError("a plugin tag must have a nonempty suffix")
        # Preserve the existing plugin-tag convention: a root plugin project.
        # A nested plugin is selected explicitly through workflow_dispatch.
        requested_path = "."
    elif event_name != "workflow_dispatch":
        raise ValueError("unsupported plugin publication event")

    # No expression/shell/output delimiters, Windows drives, or traversal.
    # Spaces within a component are supported and all consumers quote the path.
    if (
        not requested_path
        or requested_path != requested_path.strip()
        or not re.fullmatch(r"[A-Za-z0-9_./ -]+", requested_path)
    ):
        raise ValueError("plugin_path must be a plain relative checkout path")
    relative = PurePosixPath(requested_path)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValueError("plugin_path must stay inside the checkout")

    root = Path(workspace).resolve(strict=True)
    selected = root
    for part in relative.parts:
        selected /= part
        if selected.is_symlink():
            raise ValueError("plugin_path must not traverse symlinks")
    selected = selected.resolve(strict=True)
    if not selected.is_relative_to(root) or not selected.is_dir():
        raise ValueError("plugin_path must name a directory inside the checkout")
    for filename in ("plugin.yaml", "Cargo.toml"):
        manifest = selected / filename
        if manifest.is_symlink() or not manifest.is_file():
            raise ValueError(f"plugin request requires a regular {filename} in plugin_path")
    return selected.relative_to(root).as_posix()


def main():
    try:
        selected = plugin_directory(
            os.environ["GITHUB_WORKSPACE"],
            os.environ["GITHUB_EVENT_NAME"],
            os.environ["GITHUB_REF"],
            os.environ.get("INPUT_PLUGIN_PATH", "."),
        )
        # Validate completely before writing any successful output. The path
        # grammar above cannot inject a second GitHub output or shell command.
        with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
            output.write(f"requested={'false' if selected is None else 'true'}\n")
            output.write(f"plugin_path={selected or ''}\n")
    except (KeyError, OSError, ValueError) as exc:
        print(f"Plugin publication request rejected: {exc}", file=sys.stderr)
        return 1
    if selected is None:
        print("No plugin publication requested by this push.")
    else:
        print(f"Plugin publication request selects: {selected}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
