#!/usr/bin/env python3
"""Hookify PreToolUse hook — evaluates file/tool rules before tool execution."""

import json
import os
import sys

# Add parent dirs to path for imports
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

    # Extract file_path from tool_input for file-based rules
    tool_input = context.get("tool_input", {})
    file_path = tool_input.get("file_path", "")
    tool_name = context.get("tool_name", "")
    command = tool_input.get("command", "")

    # Build evaluation context
    eval_context = {
        "file_path": file_path,
        "tool_name": tool_name,
        "command": command,
        "tool_input": tool_input,
    }

    # Evaluate file-event rules
    triggered = evaluate_rules("file", eval_context)

    # Also evaluate tool-event rules
    triggered.extend(evaluate_rules("tool", eval_context))

    if not triggered:
        sys.exit(0)

    output, has_block = format_output(triggered)
    if output:
        print(output)

    sys.exit(2 if has_block else 0)


if __name__ == "__main__":
    main()
