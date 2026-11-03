"""Managed CVE lab nuclei operations."""

import hashlib
import json
import re
from cve_lab_catalog import CATALOG, LABEL, SIGNATURE
from cve_lab_engine import command, owned_remove


def template(token):
    # This template emits a transport observation for any HTTP response. Only
    # classify() decides protection; a Nuclei match here is NOT a vulnerability.
    return f"""id: mockforge-nextjs-29927-observation
info:
  name: MockForge Next.js controlled lab observation
  author: mockforge
  severity: info
http:
  - method: GET
    path:
      - "{{{{BaseURL}}}}/?__mockforge_cve={token}"
    headers:
      X-Mockforge-Cve: "{token}"
      x-middleware-subrequest: "{SIGNATURE}"
    redirects: false
    matchers:
      - type: dsl
        dsl:
          - "status_code >= 100 && status_code <= 599"
"""


def probe(engine, base, output, phase, run_id):
    token = run_id + "-" + phase
    path = output / f"{phase}.yaml"
    path.write_text(template(token))
    name = "mockforge-cve-client-" + token
    try:
        result = command(
            [
                engine,
                "run",
                "--rm",
                "--name",
                name,
                "--label",
                f"{LABEL}={run_id}",
                "--network=host",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--memory=512m",
                "--pids-limit=128",
                "-v",
                f"{output.resolve()}:/work:ro",
                CATALOG["runner_image"],
                "-u",
                base,
                "-t",
                f"/work/{phase}.yaml",
                "-duc",
                "-ni",
                "-no-stdin",
                "-silent",
                "-jsonl",
                "-timeout",
                "5",
                "-retries",
                "0",
                "-rl",
                "1",
                "-c",
                "1",
            ],
            timeout=90,
        )
        (output / f"{phase}.jsonl").write_text(result.stdout)
        (output / f"{phase}.log").write_text(result.stderr)
        findings = [
            json.loads(line)
            for line in result.stdout.splitlines()
            if line.startswith("{")
        ]
        if (
            len(findings) != 1
            or findings[0].get("template-id") != "mockforge-nextjs-29927-observation"
        ):
            raise RuntimeError(
                f"{phase}: Nuclei did not produce exactly one HTTP observation"
            )
        raw = findings[0].get("response", "")
        match = re.match(r"HTTP/\S+ (\d{3})", raw)
        if not match:
            raise RuntimeError(f"{phase}: Nuclei response evidence is missing")
        return {
            "token": token,
            "status": int(match[1]),
            "dashboard": "Dashboard Content" in raw,
            "response_sha256": hashlib.sha256(raw.encode()).hexdigest(),
        }
    finally:
        owned_remove(engine, name, run_id)
