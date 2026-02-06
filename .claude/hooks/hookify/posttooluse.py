#!/usr/bin/env python3
"""Hookify PostToolUse hook — evaluates rules after tool execution."""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from core.rule_engine import evaluate_rules, format_output


def main():
    try:
        stdin_data = sys.stdin.read()
        if not stdin_data.strip():
            sys.exit(0)
        context = json.loads(stdin_data)
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)

    tool_input = context.get("tool_input", {})
    tool_output = context.get("tool_output", "")

    eval_context = {
        "file_path": tool_input.get("file_path", ""),
        "tool_name": context.get("tool_name", ""),
        "command": tool_input.get("command", ""),
        "tool_input": tool_input,
        "tool_output": str(tool_output)[:1000],  # Truncate for performance
    }

    triggered = evaluate_rules("post_tool", eval_context)

    if not triggered:
        sys.exit(0)

    output, has_block = format_output(triggered)
    if output:
        print(output)

    sys.exit(2 if has_block else 0)


if __name__ == "__main__":
    main()
