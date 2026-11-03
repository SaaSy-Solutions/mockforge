"""Reviewed managed-lab versions and immutable image references."""

CVE = "CVE-2025-29927"
CATALOG = {
    "schema_version": 1,
    "pack_revision": 1,
    "cve": CVE,
    "title": "Next.js middleware authorization bypass",
    "platform": "Linux, local Docker/Podman engine, amd64",
    "application_version": "Next.js 15.2.2 (Vulhub)",
    "application_image": "docker.io/vulhub/nextjs@sha256:f01608b225bcb7952f5e558c445b9bd76a3bfb385e11521d1a06ad4cbe08b4a8",
    "runner_version": "Nuclei 3.11.1",
    "runner_image": "docker.io/projectdiscovery/nuclei@sha256:582d5546902e67052097cb2d07296c642d50a1afc5e44623cb038845df9a32eb",
    "references": [
        "https://github.com/vulhub/vulhub/tree/master/next.js/CVE-2025-29927",
        "https://github.com/advisories/GHSA-f82v-jwr5-mffw",
    ],
    "coverage": "One reviewed authorization-bypass case and benign controls; not all variants",
}
SIGNATURE = "middleware:middleware:middleware:middleware:middleware"
LABEL = "io.mockforge.cve.run"
