"""Managed CVE lab nuclei operations."""

import hashlib
import json
import re
import subprocess
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


def probe(engine, base, output, phase, run_id, pack, insecure=False):
    """Run one Nuclei observation probe.

    ``insecure`` mirrors the CLI's --skip-tls-verify opt-in: it governs the
    Python control transport and is recorded in the report. The pinned Nuclei
    3.11.1 runner defines no skip-TLS flag ("-insecure" was never one of its
    flags, so passing it dies at startup with "flag provided but not defined")
    and does not verify TLS certificates at all - InsecureSkipVerify is
    unconditional in its HTTP client pool - so no extra runner flag is needed
    or valid here.
    """
    probe = pack["probe"]
    token = run_id + "-" + phase
    path = output / f"{phase}.yaml"
    path.write_text(template(pack, token))
    name = "mockforge-cve-client-" + token
    try:
        command(
            [
                engine,
                "create",
                "--name",
                name,
                "--label",
                f"{LABEL}={run_id}",
                "--network=host",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--memory=512m",
                "--pids-limit=128",
                pack["catalog"]["runner_image"],
                "-u",
                base,
                "-t",
                f"/tmp/{phase}.yaml",
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
                # No TLS flag: the pinned Nuclei 3.11.1 defines no skip-TLS
                # flag and never verifies certificates (see docstring).
            ],
            timeout=90,
        )
        try:
            command(
                [engine, "cp", str(path), f"{name}:/tmp/{phase}.yaml"], timeout=90
            )
        except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
            raise RuntimeError(
                f"{phase}: failed to copy Nuclei template into container: {error}"
            ) from error
        result = command(
            [engine, "start", "-a", name], timeout=90, check=False
        )
        (output / f"{phase}.jsonl").write_text(result.stdout)
        (output / f"{phase}.log").write_text(result.stderr)
        if result.returncode:
            combined_output = "\n".join(
                part for part in (result.stdout, result.stderr) if part
            )
            detail = combined_output[-3000:] if combined_output else "no output"
            raise RuntimeError(
                f"{phase}: Nuclei exited with code {result.returncode}: {detail}; "
                f"logs: {output / f'{phase}.log'} and {output / f'{phase}.jsonl'}"
            )
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
