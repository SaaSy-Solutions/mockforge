#!/usr/bin/env bash
# Retired shared-host cleanup entry point. See docs/operations/runner-cleanup.md.
# MockForge owns its CI jobs, not the shared runner host or other repos' caches.
set -euo pipefail
cat >&2 <<'MESSAGE'
Shared runner cleanup is maintained in SaaSy-Solutions/saas-platform:
  infrastructure/github-runners/host-shared/runner-host-cleanup.sh
  infrastructure/github-runners/host-shared/README.md
This MockForge entry point is retired and performs no cleanup.
Do not install it over /usr/local/bin/runner-host-cleanup.sh.
Use the infrastructure repository's reviewed deployment procedure.
MESSAGE
exit 64
