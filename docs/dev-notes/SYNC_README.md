# Local workspace export and import

`mockforge workspace export`, `import` and `activate` ship in 0.3.236 and later.
There is no `workspace sync` command; earlier versions of this page described a
proposed sync CLI that was never built.

## Commands

Use a running local MockForge admin server and a valid local admin JWT in
`MOCKFORGE_ADMIN_TOKEN`. The JWT stays in the caller's environment, not command
arguments or the export file. `--token-env` selects another variable name.
Export requires workspace-read permission; import requires settings-management
permission. Anonymous access, development identity headers, and mock tokens do
not authorize these transfer endpoints.

```sh
mockforge workspace export frontend-dev --output workspace.yaml
mockforge workspace export frontend-dev --output workspace.json --format json
mockforge workspace import workspace.yaml --workspace-id restored-frontend
```

`--admin-url` defaults to `http://localhost:9080`. Transfers accept HTTP/HTTPS
loopback addresses only, without URL credentials, query, fragment, or a path.
Redirects and proxy environment settings are disabled for transfer requests.

Export writes a complete temporary file in the destination directory and
publishes it atomically. Existing files are preserved unless `--force` is
specified. Import creates a new workspace; an existing destination workspace
is rejected rather than overwritten. Input must use `.json`, `.yaml`, or `.yml`.

## Version 1 format

The document contains `format_version`, `workspace`, and `secret_references`.
It preserves the authored workspace configuration, nested folders, requests,
response templates/status/headers/delays, priorities, and environment structure.
Configured credential values and environment values become placeholders such
as `${MOCKFORGE_WORKSPACE_EXPORT_VALUE_000000}`. Basic-auth usernames also become
references. Recognized credential fields in JSON bodies and secret-bearing URLs
are referenced; configured sensitive values copied elsewhere are referenced too.
A body containing sensitive data is referenced as a whole, preserving its bytes
when the caller supplies that complete body value during import.

The reference's position in the workspace document identifies the value to bind.
Set the listed variables locally, then explicitly opt in to resolving them:

```sh
mockforge workspace import workspace.yaml --workspace-id restored-frontend --resolve-env
```

Missing or undeclared references fail before creating a workspace. The CLI reads
only declared names in the generated export-reference namespace. The server
does not resolve references from platform environment variables. Binding values
are sent only to the authorized loopback runtime and are not written back into
the export file or printed in command output.

Object keys are ordered deterministically; meaningful vector order is retained.
Authoring timestamps are normalized to the UTC epoch in files. Execution history,
fidelity measurements, host directory-sync settings, and host encryption settings
are excluded. Imported workspace creation metadata denotes the new instance.

The format is configuration, not a backup of provider sessions, deployed cloud
resources, traffic recordings, execution history, runtime caches, or all server
settings. Authored bodies and free text remain configuration data; the codec's
redaction rules do not establish general-purpose content classification.

## HTTP activation

Import stores authored configuration. To publish its supported static HTTP
responses on the runtime's main HTTP listener, activate it separately:

```sh
mockforge workspace activate restored-frontend
```

Requests use `/workspace/restored-frontend/<authored-path>` on the HTTP port.
Activation replaces only mocks owned by that workspace, preserves unrelated
mocks, and reports failure when no HTTP runtime is configured or publication
fails. Methods, paths, priorities, response body bytes, statuses, response
headers, delays, and request match criteria are published. Authentication rules
that this static projection cannot enforce are rejected before publication.
Authentication settings still round-trip as configuration references; this
activation does not claim authentication-policy replay or arbitrary server state.

## Existing directory sync

Core also has legacy workspace persistence and directory-sync primitives. The
admin server has separate sync status/configuration/trigger/change-review routes
under `/__mockforge/workspaces/{workspace_id}/sync/`. Their directories are on the
server filesystem, not the caller's computer. Existing plain sync helpers can
retain authentication/environment values and timestamps; they are separate from
this export format. Encrypted backups are also not reviewable plain-text Git files.
No `workspace sync` command, automatic Git watcher, cloud-account export, or
complete runtime-state round trip is provided by the transfer commands.

An owned two-runtime fixture verified the actual candidate binary creates/edits
live state, exports JSON/YAML without a synthetic credential, imports into a
fresh runtime, and reproduces static status/body behavior. It also verifies
overwrite, missing binding, unknown workspace, duplicate import, and invalid
authorization failures. Review, merged-main and installed-release acceptance
remain separate gates.
