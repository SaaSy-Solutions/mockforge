# Required CodeDig analysis blocker

PR [#1478](https://github.com/SaaSy-Solutions/mockforge/pull/1478), head
`56dc1ffa2dabdcaf67d914cf6dc85ce41f9b7166`, base
`3871677f241ee8e9726ad7646fa356c0a2df04ca`, remains blocked by the completed
CodeDig analysis. This is an analysis verdict, not an analysis service outage.
The green remaining risk budget does not override the failed verdict.

## Recorded findings

[Check run 126432992138](https://github.com/SaaSy-Solutions/mockforge/runs/126432992138)
and its [three annotations](https://api.github.com/repos/SaaSy-Solutions/mockforge/check-runs/126432992138/annotations)
report these file-level failures, each at line 1:

| File | Reported complexity | Reported delta |
| --- | ---: | ---: |
| crates/mockforge-protocol/src/protocol_abstraction/mod.rs | 45 | +45 |
| scripts/dev/verify_protocol_extraction.py | 39 | +39 |
| scripts/tests/test_protocol_extraction.py | 22 | +22 |

The [bot report](https://github.com/SaaSy-Solutions/mockforge/pull/1478#issuecomment-6624502391)
marks all three high/blocking. It identifies no individual complex function,
semantic defect, security defect or failing execution. That limits the evidence;
it does not establish a passing quality gate. No inline review findings were
present during this audit.

## Attribution evidence

At this exact base/head, `git diff --find-copies-harder --summary` identifies the
new protocol abstraction module as a 99% copy from the old Core module. The old
path now contains the compatibility facade, so GitHub's files endpoint instead
reports the new path as added, with no previous_filename. The read-only source
proof passed: seven moved files and fourteen transport files, features preserved.
The reported Rust +45 therefore does not establish newly introduced branching.

Current CodeDig main `26d1eca65acd0a47f3e8cd3e01acff088c21fefa` still computes one
score over each ChangedFile.new_content, compares only that file's old_content,
uses the full score as delta when old_content is absent, and stores the filepath
as symbol_fqn. See [analyzers.rs:286](https://github.com/SaaSy-Solutions/codedig/blob/26d1eca65acd0a47f3e8cd3e01acff088c21fefa/crates/core/src/semantic_pr_gate/analyzers.rs#L286).
The underlying input parser reconstructs old/new content from diff hunks:
[diff.rs:134](https://github.com/SaaSy-Solutions/codedig/blob/26d1eca65acd0a47f3e8cd3e01acff088c21fefa/crates/core/src/diff.rs#L134).
There is no cross-file relocation provenance in this complexity calculation.

The [metric at analyzers.rs:330](https://github.com/SaaSy-Solutions/codedig/blob/26d1eca65acd0a47f3e8cd3e01acff088c21fefa/crates/core/src/semantic_pr_gate/analyzers.rs#L330)
counts decision-shaped substrings per line, rather than executable AST units.
The comment masker deliberately preserves string literals, as its
[existing test](https://github.com/SaaSy-Solutions/codedig/blob/26d1eca65acd0a47f3e8cd3e01acff088c21fefa/crates/core/src/semantic_pr_gate/comment_mask/tests.rs#L4)
asserts. Python fixture strings such as the Rust `if` snippets in
`test_protocol_extraction.py:79` can therefore contribute to this aggregate.
Independent Python helpers and test cases also share one reported score. This
explains why 39/22 are not function-level diagnoses. Default warning/failure
thresholds are 10/20; no threshold adjustment was made.

## Disposition and next step

Existing CodeDig [issue #922](https://github.com/SaaSy-Solutions/codedig/issues/922)
already tracks complete-source hydration and executable-unit complexity. Do not
create a duplicate issue, split files merely to evade an aggregate count, or
rewrite unchanged production policy to satisfy a relocation attribution error.
A separately reviewed tool fix should preserve configured thresholds and prove
unchanged moved callables, a real complexity increase inside a moved callable,
new genuinely complex functions, fixture literals, and copies whose original
implementation remains. Source provenance must come from bounded exact-SHA
snapshots rather than caller prose or an unverified replacement diff.

No control was waived, finding dismissed, threshold changed, check status
edited, reanalysis submitted, service deployed, or external message posted in
this audit. Existing supported reanalysis paths refetch the provider diff; no
paired-origin submission field was found. Required CodeDig analysis remains
failed until normal qualification resolves it. Other required checks retain
all their existing requirements.
