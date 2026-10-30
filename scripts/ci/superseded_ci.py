"""Bind a cancellation plan to an open, trusted PR and an obsolete CI head."""
from superseded_ci_jobs import Refused, completion_tail

def eligible(repo, pull, run, jobs, expected_head):
    head = pull.get("head", {})
    if pull.get("state") != "open" or head.get("sha") != expected_head:
        raise Refused("PR is closed or its current head changed")
    if head.get("repo", {}).get("full_name") != repo:
        raise Refused("Only a trusted same-repository PR is eligible")
    if run.get("head_repository", {}).get("full_name") != repo:
        raise Refused("Fork workflow runs are not eligible")
    if run.get("event") != "pull_request" or run.get("path", "").split("@")[0] != ".github/workflows/ci.yml":
        raise Refused("Only the CI pull-request workflow is eligible")
    if run.get("head_branch") != head.get("ref") or run.get("head_sha") in (None, expected_head):
        raise Refused("Run does not belong to a superseded head of this PR branch")
    if pull.get("number") not in [item.get("number") for item in run.get("pull_requests", [])]:
        raise Refused("Run is not associated with this PR")
    if run.get("status") not in ("queued", "pending", "in_progress"):
        raise Refused("Run is already terminal")
    job_id = completion_tail(jobs)
    return {"run_id": run["id"], "old_head": run["head_sha"], "current_head": expected_head,
            "completion_job_id": job_id, "eligible": True}
