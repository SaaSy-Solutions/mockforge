"""Check actual Linux build-temp allocation, including per-user quotas.

Filesystem free space alone cannot detect EDQUOT. Allocate and release one
owned anonymous temporary file; never remove existing files or change quotas.
This proves bounded allocation now, not capacity for the entire build.
"""
import argparse
import errno
import json
import os
from pathlib import Path
import tempfile

PROBE_BYTES = 1024 * 1024
CHUNK_BYTES = 64 * 1024


def inspect(directory: Path) -> dict:
    errors = []
    allocated = 0
    try:
        directory = directory.resolve(strict=True)
        with tempfile.TemporaryFile(dir=directory) as probe:
            payload = b"\0" * CHUNK_BYTES
            while allocated < PROBE_BYTES:
                remaining = memoryview(payload)[:min(CHUNK_BYTES, PROBE_BYTES - allocated)]
                while remaining:
                    written = os.write(probe.fileno(), remaining)
                    if written <= 0:
                        raise OSError(errno.EIO, "temporary allocation made no progress")
                    allocated += written
                    remaining = remaining[written:]
            os.fsync(probe.fileno())
    except OSError as error:
        code = errno.errorcode.get(error.errno, "OSERROR")
        errors.append("build temporary directory allocation failed: " + code)
    return {"temporary_directory": str(directory), "probe_bytes": PROBE_BYTES,
            "allocated_bytes": allocated, "errors": errors}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path,
                        default=Path(os.environ.get("TMPDIR") or "/tmp"))
    args = parser.parse_args()
    result = inspect(args.directory)
    print(json.dumps(result, sort_keys=True))
    for error in result["errors"]:
        print("::error::" + error + "; inspect the configured TMPDIR and user quota before retrying")
    return int(bool(result["errors"]))


if __name__ == "__main__":
    raise SystemExit(main())
