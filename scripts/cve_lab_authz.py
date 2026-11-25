"""Authorization attestation for CVE lab runs that send requests to an operator target."""

import datetime
import os
import sys

ENV_VAR = "MOCKFORGE_CVE_AUTHORIZED"
_TRUTHY = {"1", "true", "yes"}


def _utc_now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def _granted_by(target, assume_yes):
    if assume_yes:
        return "flag"
    if os.environ.get(ENV_VAR, "").strip().lower() in _TRUTHY:
        return "env"
    if sys.stdin is not None and sys.stdin.isatty():
        answer = input(
            f"Do you have authorization to test {target}? "
            "The lab will send requests to it. Type 'yes' to confirm: "
        )
        if answer.strip().lower() == "yes":
            return "interactive"
    return None


def require_authorization(target, assume_yes=False, operator=None):
    """Return an attestation dict, or exit with status 2 when not attested."""
    granted_by = _granted_by(target, assume_yes)
    if granted_by is None:
        print(
            f"Refusing to run: no authorization attested for target {target}. "
            f"Pass --i-am-authorized, set {ENV_VAR}=1, or confirm at the prompt "
            "when run from a terminal. Only test systems you own or are "
            "explicitly authorized to test.",
            file=sys.stderr,
        )
        sys.exit(2)
    return {
        "authorized": True,
        "target": target,
        "granted_by": granted_by,
        "operator": operator or os.environ.get("USER") or "unknown",
        "attested_at": _utc_now(),
    }
