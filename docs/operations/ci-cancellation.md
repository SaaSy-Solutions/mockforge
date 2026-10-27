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


## Distinguish recovery from ordinary backlog

A missing downstream check is not automatically a stale concurrency tail. Inspect
the current head's jobs first. `Security Audit` depends on `Detect change scope`;
it is not created while that prerequisite is waiting for a runner. Registry E2E
similarly waits for `Detect registry scope`. Preserve ordinary current-head jobs.
The existing PR registry path filter can legitimately skip an unrelated tooling
change; that is not evidence that E2E tests executed. When actual E2E evidence is
required, use one existing `registry-e2e.yml` manual dispatch on the verified
current branch after pool capacity permits, and record the dispatched head SHA.
Do not change the path filter or accept a skipped check as an executed test.

### October 24 recovery verification

The previously reported #1388 tail, run `40009189276` / job `120690124000`, finished
before the recovery attempt: it was assigned at 02:06:54 UTC and completed at
02:06:59; the run finished cancelled at 02:07. The guarded dry run consequently
refused it as terminal. A fresh audit of 15 queued PR CI runs found no eligible
unassigned superseded completion tails: fourteen were current heads, and one had
ordinary unfinished work. No run was cancelled by this follow-up.

The #1350 Rust default failure on run `39791120339`, job `119930573431`, has a
runner annotation reporting `No space left on device` for its worker diagnostic
log. The dependent Test verdict correctly failed rather than reporting success.
This was an infrastructure failure, not an observed failure of the cancellation
operator: its seven guard tests, seventeen routing regressions and ten toolchain
checks passed again locally.

Read-only inspection through the existing trusted public SSH alias at 03:55 UTC
found the FSN01 root filesystem **301 GiB, 257 GiB used, 32 GiB free**, 27% inode
use, and no second disk or meaningful unpartitioned capacity. The host exposed
16 CPUs and about 26 GiB available RAM. Both Mockforge runner services were busy,
with a 14 GiB memory limit each. Their journal showed completed jobs followed by
new work; the pool was servicing its backlog. Historical disk exhaustion does not
establish that the host is still full. The runner API omitted custom `rust`
labels despite recently assigned Rust jobs; this alone does not justify changing
runner labels or trusted routing.

### Bounded next actions

1. Preserve active workers and their existing timeouts. Recheck current jobs,
   runner assignment, filesystem capacity and the PR head before any retry.
2. After a worker slot and disk headroom permit, rerun only a failed Rust job
   **on the current #1350 head** and its required dependents. If a new commit has
   already started replacement CI, wait for that run instead. The historical
   disk-full run must not be rerun after the PR head changes. Refresh the head,
   run source SHA and database job ID first:

   ```bash
   gh pr view 1350 --repo SaaSy-Solutions/mockforge --json headRefOid
   gh run view CURRENT_RUN_ID --repo SaaSy-Solutions/mockforge --json headSha,jobs
   gh run rerun CURRENT_RUN_ID --repo SaaSy-Solutions/mockforge --job JOB_DATABASE_ID
   ```

   Execute the last command only after the run SHA matches the live PR head and
   the selected job has actually failed. Do not reset the whole workflow or launch
   duplicate heavy runs. A queued retry or accepted request is not a passing
   validation result.
3. Let #1388's normal scope job release its real Security Audit job. If the
   registry PR lane skips by its existing filter and E2E execution is required,
   dispatch exactly once on the freshly verified branch, then verify that the
   resulting run recorded that same head and actually ran both fixture lanes.
4. If space again becomes insufficient, stop requesting new heavy validations.
   Preserve existing caches, worktrees and runner state. Recheck bytes and inodes;
   do not invoke cleanup hooks manually, prune Docker, or delete targets.
5. A further capacity remedy requires explicit operator approval: provision an
   additional build volume, then plan a quiescent, verified copy of runner build
   storage with permissions and rollback preserved. Current disk inventory offers
   no meaningful in-place partition expansion. Determine the required volume size
   from actual quiet-window target/container usage before requesting it. This
   runbook does not authorize allocation, migration, service stops or host changes.
