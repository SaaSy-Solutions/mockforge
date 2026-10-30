"""Check that cancellation cannot discard assigned or ordinary CI work."""

class Refused(ValueError):
    pass

def completion_tail(jobs):
    remaining = [job for job in jobs if job.get("status") != "completed"]
    if len(remaining) != 1:
        raise Refused("Expected exactly one unfinished completion job")
    job = remaining[0]
    if job.get("name") != "Test (1.96.0)" or job.get("status") != "queued":
        raise Refused("Unfinished job is not the queued legacy completion verdict")
    if job.get("runner_name") or job.get("runner_id") or job.get("runner_group_id"):
        raise Refused("A runner has already been assigned")
    if not {"self-hosted", "linux", "x64", "rust"}.issubset(job.get("labels", [])):
        raise Refused("Unexpected runner lane")
    if not any(item.get("status") == "completed" and item.get("conclusion") == "cancelled" for item in jobs):
        raise Refused("No cancelled prerequisites establish a partially cancelled run")
    return job["id"]
