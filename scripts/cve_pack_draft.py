"""Draft a profile's rule signature automatically, with the schema as the
trust boundary.

A drafter proposes the variable parts of a profile (route, signature tokens,
expected block status, controls) from a CVE's advisory. The advisory text is
untrusted input and a drafter may be an LLM, so nothing it returns is trusted:
every draft is rebuilt with the authoritative cve/title/references from the
feeds and then passed through cve_pack_profile.validate(). Because the schema
admits only inert marker values and has no body field, a draft can never yield
a runnable payload — a prompt-injected or careless draft is rejected and
flagged for review, never silently accepted.
"""

import json
import subprocess

from cve_pack_profile import ProfileError, validate

PROPOSAL_KEYS = {"route", "signature", "expect", "controls"}
MAX_ADVISORY_CHARS = 20000


class StubDrafter:
    """Returns a fixed proposal; for tests and offline demos."""

    def __init__(self, proposal):
        self._proposal = proposal

    def draft(self, context):
        return dict(self._proposal)


class CommandDrafter:
    """Runs a configured command, provider-agnostic. The command receives the
    context as JSON on stdin and must print a JSON proposal on stdout. No
    network, model or key is assumed here — wire any LLM CLI behind it."""

    def __init__(self, command, timeout=120):
        self.command = command
        self.timeout = timeout

    def draft(self, context):
        result = subprocess.run(
            self.command,
            shell=True,
            input=json.dumps(context),
            capture_output=True,
            text=True,
            timeout=self.timeout,
        )
        if result.returncode != 0:
            raise RuntimeError(f"drafter command failed: {result.stderr[:200]}")
        return json.loads(result.stdout)


def context_for(cve, *, title, references, cwes, advisory_text):
    """The untrusted-input bundle a drafter sees. Advisory text is bounded."""
    return {
        "cve": cve,
        "title": title,
        "references": references,
        "cwes": cwes,
        "advisory_text": (advisory_text or "")[:MAX_ADVISORY_CHARS],
        "instructions": (
            "Propose ONLY route, signature (method GET; header/query NAMES the "
            "rule matches, with short inert marker VALUES using [A-Za-z0-9_.:-]; "
            "no payloads, no request body), expect.blocked_status (4xx/5xx) with "
            "origin 'absent', and 1-3 benign controls expecting 200 and origin "
            "'seen'. Values must be inert markers, never working exploit strings."
        ),
    }


def draft_profile(cve, *, title, references, cwes, advisory_text, drafter):
    """Return (profile, None) when the draft validates, else (None, reason).

    The feed-authoritative cve/title/references always win over the drafter's,
    so a drafter cannot fabricate identity or inject extra reference URLs."""
    try:
        proposal = drafter.draft(
            context_for(
                cve,
                title=title,
                references=references,
                cwes=cwes,
                advisory_text=advisory_text,
            )
        )
    except (subprocess.SubprocessError, OSError, ValueError, RuntimeError) as exc:
        return None, f"drafter error: {type(exc).__name__}: {exc}"
    if not isinstance(proposal, dict) or not set(proposal) <= PROPOSAL_KEYS:
        return None, "drafter must propose only route, signature, expect, controls"
    candidate = {
        "schema_version": 1,
        "cve": cve,
        "title": title,
        "references": references,
        "route": proposal.get("route"),
        "signature": proposal.get("signature"),
        "expect": proposal.get("expect"),
        "controls": proposal.get("controls"),
    }
    try:
        return validate(candidate), None
    except ProfileError as exc:
        return None, f"schema rejected the draft: {exc}"
