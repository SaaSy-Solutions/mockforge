"""Read-only traversal of a checkout target; never follow symlink entries."""
import os
from pathlib import Path
import stat


def raise_io(error: OSError) -> None:
    raise error


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
            if not stat.S_ISREG(info.st_mode):
                errors.append("cache contains a nonregular file")
                continue
            yield path, info
