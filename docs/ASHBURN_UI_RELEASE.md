# MockForge application UI on Ashburn

The `app.mockforge.dev` application is a cloud-mode Vite bundle. Its API target
is `https://api.mockforge.dev`; the marketing site can remain on Pages. The
Ashburn Caddy route serves `/var/lib/saasy/mockforge-ui/current` with SPA
fallback after SaaS platform [PR #9712](https://github.com/SaaSy-Solutions/saas-platform/pull/9712)
is merged and deployed. Keep the existing Pages workflow as a rollback path until the public
Ashburn app and a real API flow have been verified.

## Build and release

From a clean checkout whose `HEAD` is exactly the protected `origin/main`:

```sh
scripts/deploy-ashburn-ui.py
```

The script fetches `main`, checks the canonical MockForge origin and GitHub
branch protection, then uses Corepack's pinned pnpm 10.15.0 to run a frozen
install and a cloud-mode Vite build. It sets `VITE_MOCKFORGE_MODE=cloud` and
`VITE_API_BASE_URL=https://api.mockforge.dev` explicitly. It packages `dist`
as a deterministic gzip tar archive and uploads only that archive and the
release installer modules to `saasy-ash-01`. No build runs on Ashburn.

The root-owned installed tree is:

```text
/var/lib/saasy/mockforge-ui/
  releases/<full-main-sha>/
    artifact.tar.gz
    manifest.json              # source SHA, archive SHA256, API target
    dist/index.html
    dist/assets/...
  current -> releases/<sha>/dist
  previous -> releases/<prior-sha>/dist
```

The installer verifies the archive checksum and safe member paths, rejects
symlinks and hard links within the archive, refuses a different artifact at an
existing SHA, and makes the root-owned release files read-only. Existing
release files and pointers must remain root-owned and not writable by Caddy.
It then switches `current` with an atomic symlink replacement. Repeating the
same SHA and artifact is idempotent. It records the old target as `previous`.
The script reports the new target; inspect the manifest and Caddy route on
Ashburn before changing `app.mockforge.dev` DNS. This script does not change
Caddy, DNS, or the Pages deployment.

## Rollback

From the same clean protected-main checkout:

```sh
scripts/deploy-ashburn-ui.py --rollback
```

This atomically swaps `current` to `previous` and retains the displaced target
as the new `previous`, so the operation is reversible. The rollback does not
rebuild or delete any release. If the Ashburn route itself fails, restore the
Pages DNS record separately; the existing Pages workflow remains available.

For read-only host verification:

```sh
ssh saasy-ash-01 'readlink /var/lib/saasy/mockforge-ui/current; cat /var/lib/saasy/mockforge-ui/releases/*/manifest.json'
```
