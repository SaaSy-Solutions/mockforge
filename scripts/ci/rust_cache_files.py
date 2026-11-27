"""Read-only traversal; reject external links and symlink directories."""
import os
from pathlib import Path
import re
import stat


def raise_io(error: OSError) -> None:
    raise error


PACKAGE = r"[\w-]+"
DIGEST = r"[0-9a-f]{16}"


def in_build_output(directory: Path, target: Path) -> bool:
    """Whether `directory` lies in what a package's build script was given.

    Cargo gives each package's build output a directory of its own below
    `build`. Until Cargo 1.99 it was named `build/<package>-<digest>`; from
    Cargo 1.100 it is `build/<package>/<digest>`. What the build script wrote
    is below `out` in both, and a build script makes directories there.
    """
    parts = directory.relative_to(target).parts
    for i, part in enumerate(parts):
        if part != "build":
            continue
        rest = parts[i + 1 :]
        if len(rest) >= 2 and re.fullmatch(PACKAGE + "-" + DIGEST, rest[0]) and rest[1] == "out":
            return True
        if (
            len(rest) >= 3
            and re.fullmatch(PACKAGE, rest[0])
            and re.fullmatch(DIGEST, rest[1])
            and rest[2] == "out"
        ):
            return True
    return False


def sibling_library_alias(path: Path, target: Path) -> bool:
    """Accept native build-output .so aliases to regular versioned siblings.

    librdkafka's build creates these linker aliases. Read the link text and
    lstat its sibling, without resolving directory links or following chains.
    Rust deps/fingerprints and arbitrary cache links remain subject to the
    original checks.
    """
    if not in_build_output(path.parent, target):
        return False
    if not re.fullmatch(r"lib[\w+.-]+\.so", path.name):
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
                errors.append("cache contains a directory symlink: " + (root / name).relative_to(target).as_posix())
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
                # The entry is named, relative to the target, as the job log is
                # the only view of a runner's disk.
                errors.append("cache contains a nonregular file: " + kind + ": " + path.relative_to(target).as_posix())
                continue
            yield path, info
