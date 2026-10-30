"""Closed safe canary definitions and strict expectation schema validation."""

import json
from pathlib import Path


CASES = {
    "normal-control": ("/security-rule-probe", {}),
    "nextjs-benign": ("/security-rule-probe", {"X-Client-Note": "middleware"}),
    "nextjs-header": (
        "/security-rule-probe",
        {"x-middleware-subrequest": "middleware:" * 4 + "middleware"},
    ),
    "camel-benign": ("/security-rule-probe?beanMethodName=mockforgeCanary", {}),
    "camel-parameter": ("/security-rule-probe?cAmelBeanMethodName=mockforgeCanary", {}),
    "camel-header": (
        "/security-rule-probe",
        {"cAmelBeanMethodName": "mockforgeCanary"},
    ),
}


CONTROLS = {"normal-control", "nextjs-benign", "camel-benign"}


def load_pack(path):
    pack = json.loads(Path(path).read_text())
    if not isinstance(pack, dict) or set(pack) != {"schema_version", "cases"}:
        raise ValueError("pack requires only schema_version and cases")
    if type(pack["schema_version"]) is not int or pack["schema_version"] != 1:
        raise ValueError("unsupported schema_version")
    cases = pack["cases"]
    if not isinstance(cases, list) or not 5 <= len(cases) <= len(CASES):
        raise ValueError("pack requires five supported cases and optional camel-header")
    seen = set()
    for case in cases:
        if not isinstance(case, dict) or set(case) != {"id", "status", "origin"}:
            raise ValueError("case requires only id, status and origin")
        name, status, origin = case["id"], case["status"], case["origin"]
        if not isinstance(name, str) or name not in CASES or name in seen:
            raise ValueError("unknown or duplicate case id")
        seen.add(name)
        if type(status) is not int or not 200 <= status <= 599:
            raise ValueError("status must be an integer from 200 to 599")
        if origin not in ("seen", "absent"):
            raise ValueError("origin must be seen or absent")
        if name in CONTROLS and (status != 200 or origin != "seen"):
            raise ValueError("benign controls require 200 and origin seen")
        if origin == "absent" and status < 400:
            raise ValueError("rejection requires an explicit error status")
    if not set(CASES) - {"camel-header"} <= seen:
        raise ValueError("all supported probes and benign controls are required")
    return cases
