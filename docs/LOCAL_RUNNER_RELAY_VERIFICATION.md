# Local runner and relay verification

`scripts/verify-local-runner-relay.py` exercises already built images without
using production services, customer data, providers or billing. Supply immutable
image references and the qualified runner/relay binary hashes. Images must
already be cached on the local Docker daemon; the script does not pull or build.

```bash
python3 scripts/verify-local-runner-relay.py \
  --runner-image <qualified-runner-image> --runner-binary-sha256 <hash> \
  --relay-image <qualified-relay-image> --relay-binary-sha256 <hash> \
  --redis-image <cached-redis-image> --helper-image <cached-node-image> \
  --receipt /tmp/local-runner-relay-proof.json
```

The Node helper runs only the committed synthetic callback collector and echo
backend. It does not start the helper image's application. The fixture creates
one labelled internal network and five owned containers with no published ports,
read-only roots, dropped capabilities, bounded memory/CPU/PIDs and zero swap.
Cleanup verifies exact container identity, image and ownership label before
removal. It never searches for or reaps unrelated fixtures.

The real worker consumes two jobs from a disposable Redis list. A two-step
integration job must send actual GET and POST requests with a nonce, validate
their responses, emit `step_passed` events and finish `passed`. An intentionally
wrong status assertion must emit `step_failed` and finish `failed`. The collector
requires its synthetic bearer token. It accepts repeated start callbacks like
the real registry's documented idempotent queued-to-running transition.

The worker shares only the owned helper's network namespace. Its target is
`127.0.0.1`, and `MOCKFORGE_SSRF_ALLOW_LOOPBACK=1` is set only in that disposable
worker. The production policy remains unchanged. The test flag does **not** allow
private bridge addresses; an earlier fixture attempt correctly hit that guard.

The basic relay fixture registers a local destination, then requires successful
GET/POST roundtrips preserving status, headers, query parameters and body. This
proves local HTTP relay behavior, not public DNS/TLS or a remote-client tunnel.

A separate relay instance asks the synthetic registry for a reservation without
a connected destination. The authorized lookup must be observed and the request
must not reach the backend. The receipt records the actual failure status
(currently 500), rather than claiming that reservation metadata establishes a
working cloud tunnel. No production registry authentication, tenant persistence,
billing, paid/provider job or successful public tunnel is qualified here.

## Root demo image archive

`scripts/verify-root-image-archive.py` prepares the separate release-artifact
check. It never pulls an image or accesses a deployment host. Provide an archive
and measured metadata containing `source`, `ref`, `publisher_run_id`,
`manifest_digest`, `config_digest`, `rootfs_diff_ids` and `archive_sha256`.
The source/ref must name the exact root image; the GitHub run must be a completed,
successful protected-main publisher workflow.

Pass `--oci-verifier` pointing to the reviewed SaaS `core_release_images.py`
(pinned SHA256 in the script) and `--demo-verifier` pointing to the merged
`verify-bundled-demo.py`. The complete OCI graph is checked before local load;
an existing contradictory source tag refuses import. The script measures the
loaded binary and runs the six HTTP cases against the actual image bundle,
without checkout mounts. Its receipt distinguishes archive acceptance from
runtime acceptance and preserves failures. Timeout, SIGINT and SIGTERM request
child termination and allow its cleanup handler a bounded wait; the coordinator
never forcibly kills that verifier. An unresponsive child remains identified by
PID with `unresolved_cleanup` and requires operator follow-up. Source-to-manifest attribution still
requires the coordinator's independently reviewed publisher/registry evidence;
an absent OCI revision label is not fabricated.
