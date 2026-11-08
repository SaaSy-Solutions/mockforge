# Configuration-only refresh: the native runtime remains the qualified 387 image.
ARG RUNTIME_IMAGE
FROM ${RUNTIME_IMAGE}
ARG CONFIG_SOURCE
LABEL org.opencontainers.image.revision="3871677f241ee8e9726ad7646fa356c0a2df04ca" \
      io.mockforge.image.kind="example-configuration-refresh" \
      io.mockforge.runtime.source="3871677f241ee8e9726ad7646fa356c0a2df04ca" \
      io.mockforge.config.source="${CONFIG_SOURCE}" \
      io.mockforge.runtime.base.digest="sha256:d56a4da07d89f909d9d50ba1f974291f107955af253527926f7bcafb37cbba4d" \
      io.mockforge.runtime.binary.sha256="2e1d3abb38ac81d6597d1f8fb303984852db44a6c3bd779540d4f4762f87cd17"
# The driver constructs a context containing exactly these three JSON files.
COPY --chown=mockforge:mockforge examples/ /app/examples/
