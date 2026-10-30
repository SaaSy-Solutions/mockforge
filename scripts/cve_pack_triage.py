"""Decide which CVEs a load-balancer / WAF rule can act on, and rank them.

A per-CVE rule pack only makes sense for weaknesses that show up in the HTTP
request a front end sees: injection, traversal, SSRF, auth/access bypass,
deserialization, XXE, open redirect, request smuggling, malicious upload.
CWE classes outside that (memory safety, local privilege, crypto, hardware)
are filtered out.
"""

# CWE classes a request-facing rule can plausibly match on.
WEB_CWES = {
    # Injection
    "CWE-74",
    "CWE-77",
    "CWE-78",
    "CWE-79",
    "CWE-88",
    "CWE-89",
    "CWE-90",
    "CWE-91",
    "CWE-93",
    "CWE-94",
    "CWE-95",
    "CWE-96",
    "CWE-97",
    "CWE-98",
    "CWE-113",
    "CWE-116",
    "CWE-564",
    "CWE-643",
    "CWE-652",
    "CWE-917",
    # Path traversal / file inclusion
    "CWE-22",
    "CWE-23",
    "CWE-24",
    "CWE-36",
    "CWE-73",
    "CWE-434",
    # SSRF / redirect / smuggling
    "CWE-918",
    "CWE-601",
    "CWE-444",
    # Auth / access control bypass
    "CWE-285",
    "CWE-287",
    "CWE-288",
    "CWE-290",
    "CWE-302",
    "CWE-306",
    "CWE-639",
    "CWE-862",
    "CWE-863",
    "CWE-1390",
    # Deserialization / XML
    "CWE-502",
    "CWE-611",
    "CWE-827",
}


def web_relevant(cwes):
    """True if any CWE is one a request-facing rule can match."""
    return any(c in WEB_CWES for c in (cwes or []))


def rank(tips):
    """A sort key (higher first) for monitor prioritization: actively
    exploited and ransomware-linked first, then by exploitation probability."""
    score = 0.0
    if tips.get("actively_exploited"):
        score += 100
    if tips.get("known_ransomware"):
        score += 20
    epss = tips.get("epss")
    if epss:
        score += 50 * epss["score"]
    return score
