"""Reviewed managed-lab packs, versions and immutable image references.

Each entry in PACKS couples a reviewed vulnerable application with the
request that reproduces the vulnerability and the observations the evidence
classifier expects. Packs are data: landing a new pack means adding a reviewed
entry here plus the matching baseline gate, classifier and exploit steps, not generalized runner code.

Every ``application_image`` and ``runner_image`` must be pinned by digest;
``prepare`` refuses references without ``@sha256:``. A pack whose lab image
has not completed its build-and-review carries an explicit unreviewed tag and
``image_status`` so ``mockforge cve list`` states that plainly instead of
silently pulling an unpinned image.
"""

LABEL = "io.mockforge.cve.run"

import hashlib
import json

import cve_lab_payloads as payloads

# Reviewed public flow for the CVE-2026-33017 pack, in the node shape Vulhub
# documents. Langflow evaluates a CustomComponent's code while it builds the
# class, so the build job's events carry the marker this code raises. That
# proves unauthenticated code execution without running a command, touching a
# file, or reaching the network. The endpoint accepts malformed flows too (it
# answers 200 with a job id and only the job's events show the failure), so
# this shape is pinned by tests and was verified on a lab host.
LANGFLOW_PUBLIC_BUILD_MARKER = "MOCKFORGE-CVE-2026-33017"
LANGFLOW_PUBLIC_BUILD_BODY = json.dumps(
    {
        "data": {
            "nodes": [
                {
                    "id": "mockforge-marker",
                    "type": "genericNode",
                    "data": {
                        "id": "mockforge-marker",
                        "type": "CustomComponent",
                        "node": {
                            "template": {
                                "code": {
                                    "type": "str",
                                    "value": (
                                        "from lfx.custom import Component\n"
                                        "\n"
                                        "_x = exec(\"raise Exception("
                                        "'MOCKFORGE-' + 'CVE-2026-33017')\")\n"
                                        "\n"
                                        "class Marker(Component):\n"
                                        '    display_name = "Marker"\n'
                                        "    def build(self) -> None:\n"
                                        "        pass\n"
                                    ),
                                },
                                "_type": "CustomComponent",
                            },
                            "base_classes": ["CustomComponent"],
                            "display_name": "Custom Component",
                            "outputs": [
                                {"types": ["CustomComponent"], "name": "Custom Component"}
                            ],
                        },
                    },
                }
            ],
            "edges": [],
        }
    },
    separators=(",", ":"),
)

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
    # The lab harness starts every container with --cap-drop=ALL, which removes
    # CAP_NET_BIND_SERVICE, so Apache cannot bind the privileged default port 80.
    # Serve on an unprivileged port (8080) instead of weakening the harness.
    "ports.conf": (
        "# Reviewed lab configuration: bind the unprivileged port 8080 because the\n"
        "# lab harness runs with --cap-drop=ALL (no CAP_NET_BIND_SERVICE for :80).\n"
        "Listen 8080\n"
    ),
    "000-default.conf": (
        "# Reviewed lab configuration: serve the mod_php app on the unprivileged\n"
        "# port 8080 that ports.conf binds under --cap-drop=ALL.\n"
        "<VirtualHost *:8080>\n"
        "    DocumentRoot /var/www/html\n"
        "    <Directory /var/www/html>\n"
        "        AllowOverride All\n"
        "        Require all granted\n"
        "    </Directory>\n"
        "</VirtualHost>\n"
    ),
}

PACKS = {
    "CVE-2025-29927": {
        # Verified on the lab host; declared explicitly because packs without a
        # "verified" field count as unverified.
        "verified": True,
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
        # Verified on the lab host; declared explicitly because packs without a
        # "verified" field count as unverified.
        "verified": True,
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
    "CVE-2026-111497": {
        # Verified on the lab host (Docker 29.1.3, Linux amd64) with the pinned
        # images: the reviewed trigger crashes the lab application, which the
        # decoder-crash gate confirms from the container's exit.
        "verified": True,
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2026-111497",
            "title": "Node.js Brotli decoder out-of-bounds write (Node2Shell)",
            "platform": "Linux, local Docker/Podman engine, amd64",
            "application_version": "Node.js 22.23.3 (official image) with the reviewed lab application",
            "application_image": "docker.io/library/node@sha256:69a5713e7f4997e96d579aafd78734810e0dac0dd5ee4621d72c27a2891a4088",
            "runner_version": "Nuclei 3.11.1",
            "runner_image": "docker.io/projectdiscovery/nuclei@sha256:582d5546902e67052097cb2d07296c642d50a1afc5e44623cb038845df9a32eb",
            "references": [
                "https://github.com/advisories/GHSA-9v8h-qqgm-4m35",
                "https://nodejs.org/en/blog/vulnerability/october-2026-security-releases",
            ],
            "coverage": "One reviewed decoder-crash case and benign controls; not remote code execution or every exposure surface",
            "origin_port": 18082,
            "listen_address_guidance": (
                "The recorder defaults to 127.0.0.1:18082, which only accepts local "
                "connections. Testing an LB/WAAP on another machine requires "
                "--listen-address <an isolated lab interface IP>; keep that interface "
                "restricted to the test network because the recorder forwards to the "
                "vulnerable application."
            ),
        },
        # The pinned runner observes a plain healthy request; the exploit
        # request itself rides the raw prior-knowledge HTTP/2 client, because
        # the pack's attack surface is HTTP/2 ("HTTP2Shell") and its expected
        # outcome is a crash, not an HTTP response a template can match.
        "probe": {
            "template_id": "mockforge-node-111497-observation",
            "template_name": "MockForge Node Brotli controlled lab observation",
            "method": "GET",
            "path": "/health?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
            },
            "body": None,
            "signature_header": None,
            # Body marker proving the application was healthy and observable.
            "marker": "mockforge-node2shell-lab",
        },
        "app": {
            "port": "3000/tcp",
            "env": {},
            "readiness": {"path": "/health", "status": 200},
            "workdir": "/app",
            "command": ["node", "/app/server.js"],
            "mounts": [{"host": "{output}/server.js", "container": "/app/server.js"}],
        },
        # The reviewed lab application, mounted read-only into the pinned
        # official image. It serves an HTTP endpoint that decompresses request
        # bodies with the bundled Brotli decoder - the exposure surface the
        # advisory names - plus static healthy routes for the controls. The
        # decoder bug is transport-independent; the exploit request rides
        # HTTP/2 as far as the LB/WAAP and the recorder, which forwards the
        # standard hop to this application over HTTP/1.1. The app masks
        # nothing: no restarts, no crash handling beyond reporting a clean
        # decode error on a patched runtime.
        "application_script": """'use strict';
// MockForge CVE-2026-111497 managed lab application (reviewed).
const http = require('node:http');
const zlib = require('node:zlib');

// The reviewed trigger is consumed in two phases (streaming write, then a
// short final input), matching how a streaming consumer feeds the decoder:
// trigger (23068690 bytes) + 3 trailing zero bytes.
const PHASE_ONE_BYTES = 23068693;

http.createServer((req, res) => {
  const chunks = [];
  req.on('data', (chunk) => chunks.push(chunk));
  req.on('end', () => {
    const body = Buffer.concat(chunks);
    if (req.method === 'POST' && req.url.startsWith('/decode')) {
      const decoder = zlib.createBrotliDecompress();
      decoder.on('error', (error) => {
        if (!res.headersSent) {
          res.writeHead(422, { 'content-type': 'text/plain' });
          res.end(`invalid brotli: ${error.code || error.message}\\n`);
        }
      });
      const second = body.length > PHASE_ONE_BYTES ? body.subarray(PHASE_ONE_BYTES) : Buffer.alloc(0);
      const first = body.length > PHASE_ONE_BYTES ? body.subarray(0, PHASE_ONE_BYTES) : body;
      decoder.on('data', () => {});
      decoder.write(first, () => {
        if (second.length) {
          decoder.end(second, () => {
            if (!res.headersSent) {
              res.writeHead(200, { 'content-type': 'text/plain' });
              res.end('decoded\\n');
            }
          });
        } else {
          decoder.end(() => {
            if (!res.headersSent) {
              res.writeHead(200, { 'content-type': 'text/plain' });
              res.end('decoded\\n');
            }
          });
        }
      });
      return;
    }
    res.writeHead(200, { 'content-type': 'text/plain' });
    res.end('mockforge-node2shell-lab\\n');
  });
}).listen(3000, '0.0.0.0');
""",
        "controls": {
            "benign_path": "/health?__mockforge_cve={token}",
            "benign_status": 200,
            "ordinary_path": "/?__mockforge_cve={token}",
            "ordinary_status": 200,
        },
        "exploit": {
            "method": "POST",
            "path": "/decode?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
                "Content-Type": "application/brotli",
            },
            "max_forwarded_body": 48 * 1024 * 1024,
            "timeout": 60,
        },
        "baseline_gate": "decoder-crash",
        "classifier": "decoder-crash",
        "baseline_exploit": "http2-crash",
        "target_exploit": "http2-crash",
    },
    "CVE-2026-109618": {
        # Reviewer evidence on PR #1524: trigger returned "200 upload processed"
        # with no crash marker on Docker 29.1.4 / PHP 8.4.26 / Apache 2.4.69 at
        # 418c289e. Diagnosed on the pinned php:8.4.26-apache image (issue
        # #1620): rev 2's padding rode custom part-header lines, which PHP's
        # rfc1867 parser discards, so memory_limit was never approached and the
        # OOM bail-out was never reached. Rev 3 carries the padding as form-FIELD
        # VALUES, which PHP accumulates into $_POST, so the body now raises
        # "Allowed memory size ... exhausted ... in Unknown on line 0" during the
        # parse (verified live). 8.4.26 still answers that bail-out cleanly (no
        # segfault), so the pack stays hidden until the specific corruption is
        # reproduced on a lab host; see #1620 for the finish plan.
        "verified": False,
        "catalog": {
            "schema_version": 1,
            "pack_revision": 3,
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
            "coverage": "One reviewed OOM-during-input-parse corruption case on mod_php, benign controls, and the PHP_SESSION_UPLOAD_PROGRESS marker as URI, query (POST and GET), header and urlencoded-field-name signature variants; not remote code execution, not FPM, not every trigger variant",
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
            "port": "8080/tcp",
            "cap_add": ["SETUID", "SETGID"],
            "env": {},
            "readiness": {"path": "/health.php", "status": 200},
            "workdir": "/var/www/html",
            "mounts": [
                {"host": "{output}/index.php", "container": "/var/www/html/index.php"},
                {"host": "{output}/health.php", "container": "/var/www/html/health.php"},
                {"host": "{output}/upload.php", "container": "/var/www/html/upload.php"},
                {"host": "{output}/lab.ini", "container": "/usr/local/etc/php/conf.d/zz-mockforge-lab.ini"},
                {"host": "{output}/.htaccess", "container": "/var/www/html/.htaccess"},
                {"host": "{output}/ports.conf", "container": "/etc/apache2/ports.conf"},
                {"host": "{output}/000-default.conf", "container": "/etc/apache2/sites-enabled/000-default.conf"},
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
        # Data-driven signature variants (issue #1588): additional must-be-
        # blocked placements of the PHP_SESSION_UPLOAD_PROGRESS marker. The
        # source cases are Srikanth's (srikr) field requests on issue #79
        # (Nov 14, 2026), adapted to the pack's token convention; bodies are
        # pinned builders in cve_lab_payloads, and none of them carries the
        # oversized reviewed trigger, so no variant is marked as reproducing
        # directly - they are must-be-blocked signature probes. The marker's
        # multipart form-data field-name position stays the pack set's
        # existing case (the pinned oom_body_with_progress producer and
        # consumer_body consumer), so no variant duplicates it. Every variant
        # runs in the direct baseline (must reach the origin) and through the
        # LB (must be blocked), and the verdict is reported per variant.
        "signature_variants": [
            {
                # (a) marker as the first request-URI path segment (the exact
                # source case POST /PHP_SESSION_UPLOAD_PROGRESS/upload.php).
                "id": "marker-in-uri",
                "method": "POST",
                "path": "/PHP_SESSION_UPLOAD_PROGRESS/upload.php?__mockforge_cve={token}",
                "headers": {"X-Mockforge-Cve": "{token}"},
                "body_ref": "file_upload_body",
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                # (b) marker as a query parameter on a multipart POST (the
                # exact source case POST /upload.php?PHP_SESSION_UPLOAD_PROGRESS=123).
                "id": "marker-in-query",
                "method": "POST",
                "path": "/upload.php?PHP_SESSION_UPLOAD_PROGRESS=123&__mockforge_cve={token}",
                "headers": {"X-Mockforge-Cve": "{token}"},
                "body_ref": "file_upload_body",
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                # (c) marker as a query parameter on a plain GET with no body
                # (the exact source case GET /upload.php?PHP_SESSION_UPLOAD_PROGRESS=1).
                "id": "marker-in-query-get",
                "method": "GET",
                "path": "/upload.php?PHP_SESSION_UPLOAD_PROGRESS=1&__mockforge_cve={token}",
                "headers": {"X-Mockforge-Cve": "{token}"},
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                # (d) marker as a urlencoded body FIELD NAME - a body position
                # other than the multipart field name (the exact source case
                # POST /upload.php with body PHP_SESSION_UPLOAD_PROGRESS=123).
                "id": "marker-in-urlencoded-field",
                "method": "POST",
                "path": "/upload.php?__mockforge_cve={token}",
                "headers": {"X-Mockforge-Cve": "{token}"},
                "body_ref": "progress_urlencoded_field_body",
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                # (e) marker carried in a request header (name and value, so a
                # rule matching either position blocks it) - the header
                # complement to the field cases' marker-ish x-php/test headers.
                "id": "marker-in-header",
                "method": "POST",
                "path": "/upload.php?__mockforge_cve={token}",
                "headers": {
                    "X-Mockforge-Cve": "{token}",
                    "X-PHP-Session-Upload-Progress": "PHP_SESSION_UPLOAD_PROGRESS",
                },
                "body_ref": "file_upload_body",
                "expect": "blocked",
                "reproduces_directly": False,
            },
        ],
        "baseline_gate": "modphp-segfault",
        "classifier": "modphp-segfault",
        "baseline_exploit": "modphp-oom",
        "target_exploit": "modphp-oom",
    },
    "CVE-2026-109619": {
        # Reviewer evidence on PR #1524: trigger did not reproduce on Docker
        # 29.1.4 / PHP 8.4.26 / Apache 2.4.69 at 418c289e; see PR #1524. Rev 3
        # shares the 109618 producer fix: the oversized producer now reaches the
        # OOM bail-out during the parse via field-value accumulation (verified
        # live). On 8.4.26 the producer OOM followed by the same-worker consumer
        # still does not crash the later request, so this pack also stays hidden
        # pending the specific same-worker corruption on a lab host; see #1620.
        "verified": False,
        "catalog": {
            "schema_version": 1,
            "pack_revision": 3,
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
            "port": "8080/tcp",
            "cap_add": ["SETUID", "SETGID"],
            "env": {},
            "readiness": {"path": "/health.php", "status": 200},
            "workdir": "/var/www/html",
            "mounts": [
                {"host": "{output}/index.php", "container": "/var/www/html/index.php"},
                {"host": "{output}/health.php", "container": "/var/www/html/health.php"},
                {"host": "{output}/upload.php", "container": "/var/www/html/upload.php"},
                {"host": "{output}/lab.ini", "container": "/usr/local/etc/php/conf.d/zz-mockforge-lab.ini"},
                {"host": "{output}/.htaccess", "container": "/var/www/html/.htaccess"},
                {"host": "{output}/ports.conf", "container": "/etc/apache2/ports.conf"},
                {"host": "{output}/000-default.conf", "container": "/etc/apache2/sites-enabled/000-default.conf"},
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
        # Data-driven signature variants (issue #1588 follow-up): additional
        # must-be-blocked placements of the ini_set/php_value/php_admin
        # markers. The source cases are Srikanth's (srikr) field requests on
        # issue #79 (Nov 14, 2026), adapted to the pack's token convention:
        # each marker as the raw text/plain request body (POST /test), as a
        # query parameter (POST /testing?<marker>=...), and as a request-URI
        # path segment (POST /testing/<marker>/...), with the x-php/test
        # header swap the field cases carry. Bodies are pinned builders in
        # cve_lab_payloads, and none of them carries the oversized reviewed
        # trigger, so no variant is marked as reproducing directly - they
        # are must-be-blocked signature probes. The reviewed producer and
        # consumer cases stay untouched, and no variant duplicates them.
        # Every variant runs in the direct baseline (must reach the origin)
        # and through the LB (must be blocked), and the verdict is reported
        # per variant.
        "signature_variants": [
            {
                # (a) marker as the raw text/plain request body - the exact
                # source cases POST /test with body ini_set/php_value/
                # php_admin (Content-Type comes from the pinned builder).
                "id": "ini-set-in-body",
                "method": "POST",
                "path": "/test?__mockforge_cve={token}",
                "headers": {"X-PHP": "test"},
                "body_ref": "ini_set_body",
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                "id": "php-value-in-body",
                "method": "POST",
                "path": "/test?__mockforge_cve={token}",
                "headers": {"X-PHP": "test"},
                "body_ref": "php_value_body",
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                "id": "php-admin-in-body",
                "method": "POST",
                "path": "/test?__mockforge_cve={token}",
                "headers": {"X-PHP": "test"},
                "body_ref": "php_admin_body",
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                # (b) marker as a query parameter - the exact source cases
                # POST /testing?ini_set=1 (php_value=1, php_admin=true) with
                # the swapped test/x-php header; bodyless probes, so a block
                # can only come from the query position.
                "id": "ini-set-in-query",
                "method": "POST",
                "path": "/testing?ini_set=1&__mockforge_cve={token}",
                "headers": {"Test": "x-php", "Content-Type": "text/plain"},
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                "id": "php-value-in-query",
                "method": "POST",
                "path": "/testing?php_value=1&__mockforge_cve={token}",
                "headers": {"Test": "x-php", "Content-Type": "text/plain"},
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                "id": "php-admin-in-query",
                "method": "POST",
                "path": "/testing?php_admin=true&__mockforge_cve={token}",
                "headers": {"Test": "x-php", "Content-Type": "text/plain"},
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                # (c) marker as a request-URI path segment - the exact source
                # cases POST /testing/ini_set/one (php_value/two,
                # php_admin/three); bodyless probes, so a block can only come
                # from the path position.
                "id": "ini-set-in-path",
                "method": "POST",
                "path": "/testing/ini_set/one?__mockforge_cve={token}",
                "headers": {"Test": "x-php", "Content-Type": "text/plain"},
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                "id": "php-value-in-path",
                "method": "POST",
                "path": "/testing/php_value/two?__mockforge_cve={token}",
                "headers": {"Test": "x-php", "Content-Type": "text/plain"},
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                "id": "php-admin-in-path",
                "method": "POST",
                "path": "/testing/php_admin/three?__mockforge_cve={token}",
                "headers": {"Test": "x-php", "Content-Type": "text/plain"},
                "expect": "blocked",
                "reproduces_directly": False,
            },
            {
                # (d) marker carried in a request header (name and value, so
                # a rule matching either position blocks it), with no body -
                # the header complement, mirroring the 109618 set's header
                # placement.
                "id": "ini-set-in-header",
                "method": "POST",
                "path": "/test?__mockforge_cve={token}",
                "headers": {
                    "X-Ini-Set": "ini_set",
                },
                "expect": "blocked",
                "reproduces_directly": False,
            },
        ],
        "baseline_gate": "modphp-segfault",
        "classifier": "modphp-segfault",
        "baseline_exploit": "modphp-worker-sequence",
        "target_exploit": "modphp-worker-sequence",
    },
    "CVE-2026-33017": {
        # Verified on the lab host (Docker 29.1.3, Linux amd64): the direct
        # baseline answers 200 with a job id, and the job's events carry the
        # marker the reviewed code raises.
        "verified": True,
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2026-33017",
            "title": "Langflow unauthenticated code execution via the public flow build",
            "platform": "Linux, local Docker/Podman engine, amd64",
            "application_version": "Langflow 1.8.1 (Vulhub) with automatic login disabled",
            "application_image": "docker.io/vulhub/langflow@sha256:c71622963b4d837344f8c6aee2a004a8d4a023c6c4a2eff6b03ba2818e84ef49",
            "runner_version": "Nuclei 3.11.1",
            "runner_image": "docker.io/projectdiscovery/nuclei@sha256:582d5546902e67052097cb2d07296c642d50a1afc5e44623cb038845df9a32eb",
            "references": [
                "https://github.com/advisories/GHSA-vwmf-pq79-vjvx",
                "https://github.com/vulhub/vulhub/tree/master/langflow/CVE-2026-33017",
            ],
            "coverage": "One reviewed unauthenticated public flow build that carries a benign code node; the build is accepted and the job's events carry the code's marker. Not other endpoints or an authenticated session",
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
            "template_id": "mockforge-langflow-33017-observation",
            "template_name": "MockForge Langflow controlled lab observation",
            "method": "POST",
            # The public build endpoint is asynchronous: it accepts the flow and
            # answers with a job id, and any client_id cookie is enough. The
            # marker proves the unauthenticated build was accepted; the job's
            # events then carry the marker the code raises.
            "path": "/api/v1/build_public_tmp/e8b4f8c2-7a1d-4e5f-9b3c-2a6d8f0e1b4a/flow?__mockforge_cve={token}",
            "headers": {
                "X-Mockforge-Cve": "{token}",
                "Content-Type": "application/json",
                "Cookie": "client_id=mockforge-{token}",
            },
            "body": LANGFLOW_PUBLIC_BUILD_BODY,
            "signature_header": None,
            "marker": "job_id",
        },
        "app": {
            "port": "7860/tcp",
            "env": {
                # Auto-login disabled models a deployment that requires a login.
                "LANGFLOW_HOST": "0.0.0.0",
                "LANGFLOW_AUTO_LOGIN": "false",
                "DO_NOT_TRACK": "true",
            },
            "readiness": {"path": "/", "status": 200},
        },
        "controls": {
            # The version route answers the same JSON with or without a login.
            "benign_path": "/api/v1/version?__mockforge_cve={token}",
            "benign_status": 200,
            # A route that needs a login: the application's own protection.
            "ordinary_path": "/api/v1/flows/?__mockforge_cve={token}",
            "ordinary_status": 403,
        },
        "baseline_gate": "data-driven",
        "classifier": "data-driven",
    }
}

def validate_signature_variants(pack):
    """Check the data-driven signature variants a pack declares.

    Each variant is one more reviewed must-be-blocked request: data of the
    form {id, method, path, headers, body_ref | body_template, expect:
    "blocked"}. Bodies stay generated and digest-pinned (``body_ref`` names a
    builder in ``cve_lab_payloads.PINNED_BODIES``; ``body_template`` must pin
    its exact bytes with ``body_sha256``), so a run never carries unreviewed
    input. A variant marked ``reproduces_directly`` claims the pack's
    reviewed crash evidence, which only packs with a crash-evidence baseline
    exploit can honor. Malformed variants are refused here, before any
    download.
    """
    variants = pack.get("signature_variants")
    if not variants:
        return
    seen_ids = set()
    for variant in variants:
        for key in ("id", "path", "expect"):
            if key not in variant:
                raise ValueError(f"signature variant is missing {key}: {variant}")
        variant_id = variant["id"]
        if not isinstance(variant_id, str) or not variant_id:
            raise ValueError(
                f"signature variant id must be a non-empty string: {variant}"
            )
        if variant_id in seen_ids:
            raise ValueError(f"duplicate signature variant id: {variant_id}")
        seen_ids.add(variant_id)
        if variant["expect"] != "blocked":
            raise ValueError(
                f'signature variant {variant_id} must expect "blocked": '
                f'{variant["expect"]!r}'
            )
        if not isinstance(variant["path"], str) or not variant["path"].startswith("/"):
            raise ValueError(
                f"signature variant {variant_id} path must be an origin path "
                f"starting with /: {variant['path']!r}"
            )
        method = variant.get("method", "GET")
        if (
            not isinstance(method, str)
            or not method
            or method != method.strip()
            or not all(character.isalpha() or character == "-" for character in method)
        ):
            raise ValueError(
                f"signature variant {variant_id} method must be an HTTP "
                f"method token: {method!r}"
            )
        headers = variant.get("headers", {})
        if not isinstance(headers, dict) or any(
            not isinstance(name, str) or not isinstance(value, str)
            for name, value in headers.items()
        ):
            raise ValueError(
                f"signature variant {variant_id} headers must be a string mapping"
            )
        if "body_ref" in variant and "body_template" in variant:
            raise ValueError(
                f"signature variant {variant_id} must declare body_ref or "
                "body_template, not both"
            )
        if "body_template" in variant:
            template = variant["body_template"]
            if not isinstance(template, str):
                raise ValueError(
                    f"signature variant {variant_id} body_template must be a string"
                )
            if "{token}" in template:
                raise ValueError(
                    f"signature variant {variant_id} body_template must not "
                    "embed the run token; its bytes must stay exactly the "
                    "reviewed input"
                )
            if "body_sha256" not in variant:
                raise ValueError(
                    f"signature variant {variant_id} body_template requires "
                    "a pinned body_sha256"
                )
            digest = hashlib.sha256(template.encode("utf-8")).hexdigest()
            if variant["body_sha256"] != digest:
                raise ValueError(
                    f"signature variant {variant_id} body_sha256 does not "
                    "match its body_template"
                )
        if "body_ref" in variant:
            reference = variant["body_ref"]
            if reference not in payloads.PINNED_BODIES:
                raise ValueError(
                    f"signature variant {variant_id} body_ref {reference!r} "
                    "is not a reviewed, digest-pinned payload builder"
                )
        if variant.get("reproduces_directly") and "baseline_exploit" not in pack:
            raise ValueError(
                f"signature variant {variant_id} marks reproduces_directly "
                "but the pack has no crash-evidence baseline exploit to "
                "confirm it"
            )


def validate_data_driven_pack(pack):
    """Check a pack that uses the generic data-driven evidence paths.

    A pack needs no classifier code when its outcome is a plain response
    status: it names ``data-driven`` for both ``baseline_gate`` and
    ``classifier`` and declares everything else as data. Packs that declare
    ``signature_variants`` (additional must-be-blocked requests) are also
    validated here regardless of their gate, so a wrongly declared pack
    fails loudly before any run instead of producing unverifiable verdicts.
    """
    variants = pack.get("signature_variants")
    data_driven = (
        pack["baseline_gate"] == "data-driven" and pack["classifier"] == "data-driven"
    )
    if not data_driven and not variants:
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
    validate_signature_variants(pack)


# Legacy single-pack exports; existing callers and tests import these names.
CVE = "CVE-2025-29927"
CATALOG = PACKS[CVE]["catalog"]
SIGNATURE = PACKS[CVE]["probe"]["headers"]["x-middleware-subrequest"]


# Passive version-fingerprint packs. These are DETECTION ONLY: the detect
# subcommand sends one ordinary read-only ``GET /`` and compares the version
# the target itself announces in its response headers with the published
# affected ranges. Nothing here reproduces a vulnerability, so no entry
# carries an exploit, baseline_exploit or target_exploit field, and none is
# run through the lab engine or the LB-regression path. ``fixed`` maps a
# release line to the first release that contains the fix.
DETECT_PACKS = {
    "CVE-2026-109618": {
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2026-109618",
            "title": "PHP2Shell 2026: PHP core request-lifecycle corruption (passive version check)",
            "mode": "detect-only",
            "references": [
                "https://github.com/php/php-src/security/advisories/GHSA-w8hw-w9fx-j79w",
                "https://www.php.net/releases/8.4/release-8.4.27.php",
            ],
            "coverage": (
                "Version fingerprint only, from the Server and X-Powered-By headers. "
                "It does not test the vulnerability. Distribution builds can backport "
                "the fix without changing the version, and many sites hide the header."
            ),
        },
        "probe": {"method": "GET", "path": "/?__mockforge_cve={token}"},
        "fingerprint": {
            "product": "php",
            "fixed": {"8.5": "8.5.12", "8.4": "8.4.27", "8.3": "8.3.36", "8.2": "8.2.35"},
            "disclosed": "2026-10-21",
        },
        "classifier": "version-fingerprint",
    },
    "CVE-2026-109619": {
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2026-109619",
            "title": "PHP2Shell 2026: PHP core same-worker use-after-free (passive version check)",
            "mode": "detect-only",
            "references": [
                "https://github.com/php/php-src/security/advisories/GHSA-6wmf-4rv5-p3rp",
                "https://www.php.net/releases/8.4/release-8.4.27.php",
            ],
            "coverage": (
                "Version fingerprint only, from the Server and X-Powered-By headers. "
                "It does not test the vulnerability. Distribution builds can backport "
                "the fix without changing the version, and many sites hide the header."
            ),
        },
        "probe": {"method": "GET", "path": "/?__mockforge_cve={token}"},
        "fingerprint": {
            "product": "php",
            "fixed": {"8.5": "8.5.12", "8.4": "8.4.27", "8.3": "8.3.36", "8.2": "8.2.35"},
            "disclosed": "2026-10-21",
        },
        "classifier": "version-fingerprint",
    },
    "CVE-2026-111497": {
        "catalog": {
            "schema_version": 1,
            "pack_revision": 1,
            "cve": "CVE-2026-111497",
            "title": "Node2Shell: Node.js Brotli decoder out-of-bounds write (passive version check)",
            "mode": "detect-only",
            "references": [
                "https://github.com/advisories/GHSA-9v8h-qqgm-4m35",
                "https://nodejs.org/en/blog/vulnerability/october-2026-security-releases",
            ],
            "coverage": (
                "Version fingerprint only. Node.js rarely announces its version to "
                "remote clients, so the usual result is unknown. A version is read "
                "only when the application leaks it in a header such as X-Powered-By. "
                "It does not test the vulnerability."
            ),
        },
        "probe": {"method": "GET", "path": "/?__mockforge_cve={token}"},
        "fingerprint": {
            "product": "node",
            "fixed": {"22": "22.23.4", "24": "24.22.1", "26": "26.12.1"},
            # Majors below the newest fixed line that are not listed in
            # ``fixed`` (25, 23, 20 and earlier) are end of life with no fix.
            "unfixed_below_major": 27,
        },
        "classifier": "version-fingerprint",
    },
}
