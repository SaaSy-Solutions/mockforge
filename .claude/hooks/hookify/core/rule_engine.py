"""
Hookify Rule Engine — Evaluates rules against tool input context.

Supports condition types:
- regex_match: Test a field value against a regex pattern
- contains: Test if a field value contains a substring
- equals: Test if a field value exactly matches
- not_contains: Test if a field value does NOT contain a substring
"""

import re
from typing import Any

from .config_loader import load_rules


def evaluate_condition(condition: dict[str, str], context: dict[str, Any]) -> bool:
    """Evaluate a single condition against the context."""
    field = condition.get("field", "")
    operator = condition.get("operator", "regex_match")
    pattern = condition.get("pattern", "")

    # Get the field value from context (supports nested keys with dot notation)
    value = context
    for key in field.split("."):
        if isinstance(value, dict):
            value = value.get(key, "")
        else:
            value = ""
            break

    value_str = str(value) if value else ""

    if operator == "regex_match":
        return bool(re.search(pattern, value_str))
    elif operator == "contains":
        return pattern.lower() in value_str.lower()
    elif operator == "equals":
        return value_str == pattern
    elif operator == "not_contains":
        return pattern.lower() not in value_str.lower()
    else:
        return False


def evaluate_rules(
    event_type: str, context: dict[str, Any]
) -> list[dict[str, Any]]:
    """Evaluate all rules for an event type against the given context.

    Returns list of triggered rules with their messages.
    """
    rules = load_rules(event_filter=event_type)
    triggered = []

    for rule in rules:
        conditions = rule.get("conditions", [])
        if not conditions:
            continue

        # All conditions must match (AND logic)
        all_match = all(evaluate_condition(c, context) for c in conditions)

        if all_match:
            triggered.append(rule)

    return triggered


def format_output(triggered_rules: list[dict[str, Any]]) -> tuple[str, bool]:
    """Format triggered rules into hook output."""
    if not triggered_rules:
        return "", False

    messages = []
    for rule in triggered_rules:
        name = rule.get("name", "unnamed")
        message = rule.get("message", "")

        prefix = f"[hookify:{name}]"
        if message:
            messages.append(f"{prefix} {message}")
        else:
            messages.append(f"{prefix} Rule triggered")

    output = "\n".join(messages)

    # If any rule has action=block, exit with code 2
    has_block = any(r.get("action") == "block" for r in triggered_rules)

    return output, has_block
