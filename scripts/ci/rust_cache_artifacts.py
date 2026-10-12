"""Match Cargo library success fingerprints to local, nonempty outputs."""
from pathlib import Path
import re
import stat

FINGERPRINT = re.compile(r".*/\.fingerprint/(?P<package>[A-Za-z0-9_-]+-(?P<digest>[0-9a-f]{16}))/lib-(?P<crate>[A-Za-z0-9_-]+)")


def regular_nonempty(path: Path) -> bool:
    try:
        info = path.lstat()
    except FileNotFoundError:
        return False
    return stat.S_ISREG(info.st_mode) and info.st_size > 0


def declared_outputs(dep_info: Path, candidates: list[str]) -> list[str]:
    # Cached dep-info can contain the previous runner's absolute paths.
    # Match known output basenames; never open paths supplied by it.
    if not regular_nonempty(dep_info):
        return []
    if dep_info.stat().st_size > 8 * 1024 * 1024:
        raise OSError("oversized dep-info")
    text = dep_info.read_text(errors="replace")
    return [name for name in candidates if re.search(r"(?:^|[/ ])" + re.escape(name) + r":", text, re.MULTILINE)]


def library_problem(path: Path) -> tuple[int, str | None]:
    """Return fingerprint count and an optional local artifact diagnostic."""
    identity = FINGERPRINT.fullmatch(path.as_posix())
    if identity is None:
        return 0, None
    deps = path.parents[2] / "deps"
    if deps.is_symlink():
        return 1, "cache artifact directory is a symlink"
    crate = identity["crate"].replace("-", "_")
    digest = identity["digest"]
    stem = "lib" + crate + "-" + digest
    candidates = [stem + suffix for suffix in (".rlib", ".rmeta", ".so", ".a")]
    candidates += ["lib" + crate + suffix for suffix in (".so", ".a")]
    declared = declared_outputs(deps / (crate + "-" + digest + ".d"), candidates)
    if declared:
        valid = all(regular_nonempty(deps / name) for name in declared)
        diagnostic = "declared library output missing or empty: "
    else:
        valid = any(regular_nonempty(deps / name) for name in candidates)
        diagnostic = "completed library fingerprint lacks an artifact: "
    return 1, None if valid else diagnostic + identity["package"]
