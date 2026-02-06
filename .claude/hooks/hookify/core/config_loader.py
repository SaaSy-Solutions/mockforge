"""
Hookify Config Loader — Parses YAML frontmatter from hookify rule files.

Rule files are stored as `.claude/hookify.*.local.md` with YAML frontmatter
containing rule configuration and markdown body containing the message.
"""

import glob
import os
import re
import sys
from typing import Any


def find_project_root() -> str:
    """Walk up from CWD to find the git root."""
    path = os.getcwd()
    while path != "/":
        if os.path.isdir(os.path.join(path, ".git")):
            return path
        path = os.path.dirname(path)
    return os.getcwd()


def _unescape_value(val: str) -> str:
    """Unescape a YAML-like string value. Strip quotes and unescape backslashes."""
    val = val.strip().strip('"').strip("'")
    # Unescape double-backslashes (YAML-style escaping)
    val = val.replace("\\\\", "\\")
    return val


def parse_frontmatter(content: str) -> tuple[dict[str, Any], str]:
    """Parse YAML-like frontmatter from markdown content.

    Returns (metadata_dict, body_text).
    Uses simple key-value parsing to avoid PyYAML dependency.
    """
    if not content.startswith("---"):
        return {}, content

    parts = content.split("---", 2)
    if len(parts) < 3:
        return {}, content

    frontmatter_text = parts[1].strip()
    body = parts[2].strip()
    metadata: dict[str, Any] = {}

    current_key = None
    current_list: list[dict[str, str]] | None = None

    for line in frontmatter_text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        # Check for list item under a key (conditions list)
        if stripped.startswith("- ") and current_key and current_list is not None:
            # Parse nested key-value in list item
            item_str = stripped[2:].strip()
            if ":" in item_str:
                # Could be a dict-style list item
                item_dict: dict[str, str] = {}
                # Simple inline dict parsing: field: value
                for kv_part in re.split(r",\s*", item_str):
                    if ":" in kv_part:
                        k, v = kv_part.split(":", 1)
                        item_dict[k.strip()] = _unescape_value(v)
                if item_dict:
                    current_list.append(item_dict)
            continue

        # Check for continuation of list items (indented with field: value)
        if line.startswith("    ") and current_key and current_list is not None:
            kv_match = re.match(r"\s+(\w+):\s*(.+)", line)
            if kv_match and current_list:
                current_list[-1][kv_match.group(1)] = _unescape_value(
                    kv_match.group(2)
                )
            continue

        # Regular key: value
        match = re.match(r"^(\w[\w-]*):\s*(.*)", stripped)
        if match:
            key = match.group(1)
            value = match.group(2).strip()
            current_key = key
            current_list = None

            if not value:
                current_list = []
                metadata[key] = current_list
            elif value.lower() == "true":
                metadata[key] = True
            elif value.lower() == "false":
                metadata[key] = False
            else:
                metadata[key] = _unescape_value(value)

    return metadata, body


def load_rules(event_filter: str | None = None) -> list[dict[str, Any]]:
    """Load all hookify rules from .claude/hookify.*.local.md files.

    Args:
        event_filter: Optional event type to filter rules by (e.g., 'file', 'tool', 'stop', 'prompt')

    Returns:
        List of rule dicts with keys: name, enabled, event, action, conditions, message, file_path
    """
    project_root = find_project_root()
    pattern = os.path.join(project_root, ".claude", "hookify.*.local.md")
    rule_files = glob.glob(pattern)

    rules = []
    for rule_file in sorted(rule_files):
        try:
            with open(rule_file, "r") as f:
                content = f.read()

            metadata, body = parse_frontmatter(content)

            rule = {
                "name": metadata.get("name", os.path.basename(rule_file)),
                "enabled": metadata.get("enabled", True),
                "event": metadata.get("event", "file"),
                "action": metadata.get("action", "warn"),
                "conditions": metadata.get("conditions", []),
                "message": body,
                "file_path": rule_file,
            }

            # Apply event filter
            if event_filter and rule["event"] != event_filter:
                continue

            # Skip disabled rules
            if not rule["enabled"]:
                continue

            rules.append(rule)
        except Exception:
            # Skip malformed rule files silently
            continue

    return rules


if __name__ == "__main__":
    # Debug: print all loaded rules
    import json

    event = sys.argv[1] if len(sys.argv) > 1 else None
    rules = load_rules(event)
    print(json.dumps(rules, indent=2, default=str))
