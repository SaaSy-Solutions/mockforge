# Example configuration images

The manual `Example configuration image` workflow publishes a sample refresh,
using the already qualified native runtime from commit
`3871677f241ee8e9726ad7646fa356c0a2df04ca`. It does not rebuild Rust or the UI.
The base is pinned to registry manifest
`sha256:d56a4da07d89f909d9d50ba1f974291f107955af253527926f7bcafb37cbba4d`.

The distinct `examples-<config-source>` tag carries explicit runtime source,
configuration source, original base digest and original binary hash labels.
`org.opencontainers.image.revision` remains the runtime source. The binary's
embedded native build stamp also remains unchanged. Use the immutable registry
digest in `publication.json` when referring to the published artifact; a tag
alone is not an immutable reference.

Before building, the driver requires an exact, clean checkout. Every input
copied by the original Dockerfile must match the runtime commit, including the
full crate/UI, vendor, proto and examples trees, Cargo manifests, entrypoint,
configuration, Dockerfile and ignore rules. Only the three corrected scenario
OpenAPI files may differ. A native change requires a full rebuild; this workflow
refuses it rather than attributing old native code to a new source commit.

The driver requires the qualified base already cached on the local Docker
Engine. It never pulls images. It builds a context containing just the three
JSON files and the restricted Dockerfile, with no `RUN`, no network and no Rust
compilation. The disk preflight requires at least eight times the base image
size (and at least 2 GiB) on both the Docker and proof filesystems.

It checks the exported complete OCI graph, every blob hash and uncompressed
layer digest. The original twelve layers must remain unchanged, with exactly
one extra layer containing only the three expected JSON files and their parent
directories. Runtime settings must remain unchanged apart from explicit labels.
Docker compatibility-manifest config, layers and tag absence must agree with
the OCI graph. Both classic config IDs and containerd manifest IDs are accepted
only with the matching actual rootfs and Linux amd64 identity.
Classic Docker's parent-image bookkeeping and default attach/TTY serialization
are excluded from that runtime comparison. The actual new image then serves all
six HTTP examples without checkout mounts; the measured binary must match the
qualified runtime hash and container cleanup must pass.

```bash
python3 scripts/example_configuration_image.py \
  --source "$(git rev-parse HEAD)" --output /tmp/new-example-image-proof
```

This local command qualifies an image without publishing it. `--check-only`
checks the source gate without Docker. Proof directories must be new and outside
the checkout. Failures preserve their receipts and logs. The CI publish job runs
only by explicit dispatch from protected main, after the cheap source/negative
tests and actual image qualification. It publishes the measured image ID to the
distinct tag and records the registry digest separately from the local export
manifest. It does not update deployment pins, native-image tags or `latest`.

Native fixes merged after the runtime commit invalidate this narrow refresh
path. Qualify a new native base before extending the supported configuration
source range; never weaken the input gate to pass a newer runtime change.

Publication JSON and the registry digest are printed to the job log and step
summary. A short-lived artifact upload is best effort because organization
artifact storage may be full; this never weakens qualification or push gates.
