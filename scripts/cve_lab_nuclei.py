"""Managed CVE lab nuclei operations."""

import hashlib
import json
import re
from cve_lab_catalog import LABEL
from cve_lab_engine import command, owned_remove


def template(pack, token):
    """Emit the observation template for a pack's signature request.

    This template emits a transport observation for any HTTP response. Only
    classify() decides protection; a Nuclei match here is NOT a vulnerability.
    """
    probe = pack["probe"]
    headers = {
        name: value.replace("{token}", token) for name, value in probe["headers"].items()
    }
    header_lines = "\n".join(f'      {name}: "{value}"' for name, value in headers.items())
    body_block = ""
    if probe.get("body") is not None:
        body_block = f"\n    body:\n      - \"{probe['body'].replace('{token}', token)}\""
    return f"""id: {probe['template_id']}
info:
  name: {probe['template_name']}
  author: mockforge
  severity: info
http:
  - method: {probe['method']}
    path:
      - "{{{{BaseURL}}}}{probe['path'].replace('{token}', token)}"
    headers:
{header_lines}
    redirects: false{body_block}
    matchers:
      - type: dsl
        dsl:
          - "status_code >= 100 && status_code <= 599"
"""


def probe(engine, base, output, phase, run_id, pack):
    probe = pack["probe"]
    token = run_id + "-" + phase
    path = output / f"{phase}.yaml"
    path.write_text(template(pack, token))
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
                pack["catalog"]["runner_image"],
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
            or findings[0].get("template-id") != probe["template_id"]
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
            "marker": probe["marker"] in raw,
            "response_sha256": hashlib.sha256(raw.encode()).hexdigest(),
        }
    finally:
        owned_remove(engine, name, run_id)
