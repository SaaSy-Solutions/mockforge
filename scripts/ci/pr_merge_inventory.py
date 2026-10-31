#!/usr/bin/env python3
"""Print open PR dependencies and check results without changing GitHub state."""

import json
import subprocess


def main():
    fields = (
        "number,title,headRefName,baseRefName,headRefOid,isDraft,"
        "mergeStateStatus,statusCheckRollup,url"
    )
    prs = json.loads(subprocess.check_output([
        "gh", "pr", "list", "--repo", "SaaSy-Solutions/mockforge",
        "--state", "open", "--limit", "1000", "--json", fields,
    ], text=True))
    by_branch = {p["headRefName"]: p["number"] for p in prs}
    print("PR\tParent\tDraft\tMerge state\tPending\tFailed checks\tHead\tTitle")
    for pr in sorted(prs, key=lambda p: p["number"]):
        pending = []
        failed = []
        for check in pr["statusCheckRollup"]:
            name = check.get("name", check.get("context", "unknown"))
            status = check.get("status", check.get("state"))
            result = check.get("conclusion", check.get("state"))
            if status in {"QUEUED", "IN_PROGRESS", "PENDING", "EXPECTED", "WAITING"}:
                pending.append(name)
            elif result not in {"SUCCESS", "SKIPPED", "NEUTRAL"}:
                failed.append(f"{name}: {result}")
        row = [
            pr["number"], by_branch.get(pr["baseRefName"], pr["baseRefName"]),
            pr["isDraft"], pr["mergeStateStatus"], len(pending),
            "; ".join(failed), pr["headRefOid"], pr["title"],
        ]
        print("\t".join(str(value).replace("\t", " ").replace("\n", " ") for value in row))


if __name__ == "__main__":
    main()
