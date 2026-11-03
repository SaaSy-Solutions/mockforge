# Cloudflare Containers cannot pull from GHCR, so wrangler builds this one-line
# image and pushes it to Cloudflare's registry on deploy. The image's default
# command (`serve --admin`) reads the mock's configuration from the
# environment the Worker passes at start.
ARG MOCKFORGE_IMAGE=ghcr.io/saasy-solutions/mockforge:latest
FROM ${MOCKFORGE_IMAGE}
