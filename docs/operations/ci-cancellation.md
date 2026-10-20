# Superseded CI completion jobs

Trusted Rust CI deliberately retains its `always()` completion verdict. Failed,
cancelled or missing validation lanes must not produce an accepted skipped
required context. Do not remove that behavior to shorten the runner queue.

A GitHub concurrency cancellation can cancel the other jobs while leaving this
completion job queued on the existing Hetzner pool. The old workflow then holds
its concurrency slot, leaving the replacement workflow `pending` without jobs.
This was observed on superseded runs 39412998824 and 39319422249 on October 20,
2026. Both replacement workflows entered the normal queue after cleanup.

Use the guarded operator when the run is obsolete, partially cancelled, and
has no assigned or executing worker:

```bash
python3 scripts/ci/cancel_superseded_completion.py \
  --pull-request PR_NUMBER --run-id OLD_RUN_ID \
  --expected-head CURRENT_PR_HEAD_SHA
```

The default is a dry run. The operator checks the live PR head, trusted repository,
workflow path, PR association, job states, labels and runner assignment. It refuses
the current head, forks, closed PRs, unrelated workflows, ordinary queued work and
assigned workers. Every unfinished job must be the one queued legacy completion
verdict, and cancelled prerequisites must establish the partial cancellation.

After reviewing the dry-run result, add `--apply` with the same arguments. It first
requests ordinary cancellation. If that does not finish the workflow, it rechecks
all guards before requesting GitHub's
[force-cancel endpoint](https://docs.github.com/rest/actions/workflow-runs#force-cancel-a-workflow-run),
which can stop an execution retained by an `always()` condition. Existing `gh`
authentication needs permission to cancel workflow runs; credentials and response
bodies are not logged.

Verify that the old run finishes **cancelled**, then inspect the replacement run.
Entering the queue does not mean its tests passed. Run the normal required checks
on the replacement head before merging. No runner installation, extra capacity,
labels, host cleanup or CI verdict changes are involved.
