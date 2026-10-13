"""Read-only traversal; reject external links and symlink directories."""
import os
from pathlib import Path
import re
import stat


def raise_io(error: OSError) -> None:
    raise error


def sibling_library_alias(path: Path, target: Path) -> bool:
    """Accept native build-output .so aliases to regular versioned siblings.

    librdkafka's build creates these linker aliases. Read the link text and
    lstat its sibling, without resolving directory links or following chains.
    Rust deps/fingerprints and arbitrary cache links remain subject to the
    original checks.
    """
    parts = path.relative_to(target).parts
    build_output = any(
        parts[i] == "build"
        and re.fullmatch(r"[\w-]+-[0-9a-f]{16}", parts[i + 1])
        and parts[i + 2] == "out"
        for i in range(len(parts) - 3)
    )
    if not build_output or not re.fullmatch(r"lib[\w+.-]+\.so", path.name):
        return False
    name = os.readlink(path)
    if not re.fullmatch(re.escape(path.name) + r"\.\d+(?:\.\d+)*", name):
        return False
    try:
        info = (path.parent / name).lstat()
    except FileNotFoundError:
        return False
    return stat.S_ISREG(info.st_mode) and info.st_size > 0


def regular_files(target: Path, errors: list[str]):
    """Yield file metadata and report unsafe entries without modifying them."""
    if target.is_symlink():
        errors.append("target is a symlink; refusing to inspect an external cache")
        return
    if not target.exists():
        return
    if not target.is_dir():
        errors.append("target is not a directory")
        return
    for directory, directories, files in os.walk(target, followlinks=False, onerror=raise_io):
        root = Path(directory)
        for name in list(directories):
            if (root / name).is_symlink():
                directories.remove(name)
                errors.append("cache contains a directory symlink")
        for name in files:
            path = root / name
            info = path.lstat()
            if stat.S_ISLNK(info.st_mode) and sibling_library_alias(path, target):
                # The regular sibling is counted independently. Never read the
                # link as an artifact or count the same library twice.
                continue
            if not stat.S_ISREG(info.st_mode):
                kind = (
                    "unsupported file symlink" if stat.S_ISLNK(info.st_mode)
                    else "socket" if stat.S_ISSOCK(info.st_mode)
                    else "FIFO" if stat.S_ISFIFO(info.st_mode)
                    else "special file"
                )
                errors.append("cache contains a nonregular file: " + kind)
                continue
            yield path, info
