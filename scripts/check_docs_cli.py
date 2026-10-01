#!/usr/bin/env python3
"""
Docs accuracy gate: CLI invocations and HTTP paths in the mdBook.

Every `mockforge ...` (and `mockforge-plugin ...` / `mockforge-tui ...`)
command line inside a fenced code block, or inline code span, in
book/src/**/*.md is checked against the real CLI surface (subcommand tree +
flags), and every documented MockForge HTTP path (`/__mockforge/...` and
`/api/...` under a namespace MockForge itself owns) is checked against the
routes registered in the Rust sources.

CLI surface
-----------
The CLI surface lives in a committed snapshot, scripts/docs-cli-surface.json,
produced by walking `--help` of a real binary. CI checks against the snapshot
(building the full CLI is too slow for a docs job); a cheap staleness guard
compares the snapshot's top-level subcommands to the `Commands` enum in
crates/mockforge-cli/src/main.rs. Refresh the snapshot after changing CLI
flags or subcommands:

    CARGO_TARGET_DIR=/tmp/mf-docs cargo build -p mockforge-cli --bin mockforge \
        --features all-protocols,pipelines,tui,stt-cloud
    CARGO_TARGET_DIR=/tmp/mf-docs cargo build -p mockforge-plugin-cli -p mockforge-tui
    python3 scripts/check_docs_cli.py --refresh-cli /tmp/mf-docs/debug/mockforge \
        /tmp/mf-docs/debug/mockforge-plugin /tmp/mf-docs/debug/mockforge-tui

HTTP routes
-----------
Routes are collected statically from `.route("...")` / `.nest("...")` /
`.nest_service("...")` string literals in crates/*/src (test modules
excluded). A documented path matches when it equals a route, or a route
under up to two levels of nest prefixes. Path parameters (`{id}`, `:id`,
`<id>`, `*rest`) match any single segment. Concrete example values in docs
(e.g. `/api/v1/orgs/123`) match parameter segments too.

`/api/...` paths are only checked when their first segments match a
namespace MockForge registers itself (e.g. `/api/v1/...`, `/api/chaos/...`),
so user mock-API examples like `/api/users/123` are ignored.

Escape hatches
--------------
- A line containing `docs-check: ignore` (e.g. in a trailing shell comment) is
  skipped.
- A fenced block whose info string contains `ignore-docs-check` is skipped.

Usage
-----
    python3 scripts/check_docs_cli.py            # check, exit 1 on misses
    python3 scripts/check_docs_cli.py --verbose  # also print counts per page
    python3 scripts/check_docs_cli.py --refresh-cli PATH_TO_MOCKFORGE_BINARY
"""

from __future__ import annotations

import argparse
import json
import re
import shlex
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BOOK = REPO / "book" / "src"
SNAPSHOT = REPO / "scripts" / "docs-cli-surface.json"
MAIN_RS = REPO / "crates" / "mockforge-cli" / "src" / "main.rs"

# Historical records, not descriptions of the current surface.
SKIP_PAGES = {"book/src/reference/changelog.md"}

# Number of misses tolerated. Keep at 0: fix the page instead.
BASELINE = 0

SHELL_LANGS = {"", "bash", "sh", "shell", "console", "zsh", "text", "terminal", "cmd", "powershell", "ps1"}

# ---------------------------------------------------------------------------
# CLI surface snapshot
# ---------------------------------------------------------------------------

OPT_RE = re.compile(
    r"^\s{1,12}(?:(-[A-Za-z0-9?]),\s+)?(--[A-Za-z0-9][A-Za-z0-9_-]*)"
    r"(?:[ =](\[?<[^>]+>(?:\.\.\.)?\]?(?:\.\.\.)?))?"
)
SHORT_ONLY_RE = re.compile(r"^\s{1,12}(-[A-Za-z0-9?])(?:\s+(\[?<[^>]+>\]?(?:\.\.\.)?))?(?:\s{2,}|$)")
ALIAS_RE = re.compile(r"\[(?:visible )?(?:short )?aliases?: ([^\]]+)\]")


def parse_help(text: str) -> dict:
    """Parse clap help text into {"commands": {name: [aliases]}, "flags": {flag: takes_value}}."""
    commands: dict[str, list[str]] = {}
    flags: dict[str, int] = {}  # 0 = switch, 1 = takes a value, 2 = optional value
    section = None
    last_flag_names: list[str] = []
    for line in text.splitlines():
        if re.match(r"^[A-Z][A-Za-z0-9 /()&,.'-]*:\s*$", line):
            section = "commands" if line.strip() == "Commands:" else "other"
            last_flag_names = []
            continue
        if section == "commands":
            m = re.match(r"^  ([A-Za-z0-9][A-Za-z0-9_-]*)(?:,\s*[A-Za-z0-9_-]+)*(?:\s{2,}|$)", line)
            if m:
                name = m.group(1)
                commands.setdefault(name, [])
                am = ALIAS_RE.search(line)
                if am:
                    commands[name].extend(a.strip() for a in am.group(1).split(","))
                continue
            am = ALIAS_RE.search(line)
            if am and commands:
                last = list(commands)[-1]
                commands[last].extend(a.strip() for a in am.group(1).split(","))
            continue
        m = OPT_RE.match(line)
        if m:
            takes = 0 if not m.group(3) else (2 if m.group(3).startswith("[") else 1)
            names = [n for n in (m.group(1), m.group(2)) if n]
            for n in names:
                flags[n] = takes
            last_flag_names = names
            continue
        m = SHORT_ONLY_RE.match(line)
        if m and section != "commands":
            takes = 0 if not m.group(2) else (2 if m.group(2).startswith("[") else 1)
            flags[m.group(1)] = takes
            last_flag_names = [m.group(1)]
            continue
        am = ALIAS_RE.search(line)
        if am and last_flag_names:
            takes = flags[last_flag_names[0]]
            for a in am.group(1).split(","):
                a = a.strip()
                if not a.startswith("-"):
                    a = ("--" if len(a) > 1 else "-") + a
                flags[a] = takes
    return {"commands": commands, "flags": flags}


def walk_cli(binary: str, path: list[str], depth: int = 0) -> dict:
    out = subprocess.run(
        [binary, *path, "--help"], capture_output=True, text=True, timeout=60
    )
    text = out.stdout or out.stderr
    parsed = parse_help(text)
    node = {"flags": parsed["flags"], "aliases": {}, "subcommands": {}}
    um = re.search(r"^Usage: (.*)$", text, re.M)
    if um:
        # Count positionals after the command path, e.g. `[OPTIONS] <SPEC> [OUT]...`
        rest = um.group(1).split(" ")[1 + len(path):]
        pos = [t for t in rest if re.match(r"^[<\[][A-Z_0-9-]+[>\]]", t) and t not in {"[OPTIONS]", "<COMMAND>", "[COMMAND]"}]
        node["positionals"] = len(pos)
        node["variadic"] = any(t.endswith("...") for t in pos) or "--" in rest
    for name, aliases in parsed["commands"].items():
        if name == "help":
            node["subcommands"]["help"] = {"flags": {}, "aliases": {}, "subcommands": {}, "free": True}
            continue
        if depth > 6:
            continue
        node["subcommands"][name] = walk_cli(binary, [*path, name], depth + 1)
        for a in aliases:
            node["aliases"][a] = name
    return node


def refresh_snapshot(binaries: list[str]) -> None:
    data = {}
    for binary in binaries:
        name = Path(binary).name.removesuffix(".exe")
        tree = walk_cli(binary, [])
        version = subprocess.run([binary, "--version"], capture_output=True, text=True).stdout.strip()
        data[name] = {"version": version, "root": tree}
        print(f"snapshot {name}: {version}")
    SNAPSHOT.write_text(json.dumps(data, indent=1, sort_keys=True) + "\n")
    print(f"wrote {SNAPSHOT.relative_to(REPO)}")


def code_top_level_commands() -> set[str]:
    text = MAIN_RS.read_text(encoding="utf-8")
    start = text.find("enum Commands {")
    if start < 0:
        return set()
    depth = 0
    body_start = text.index("{", start)
    i = body_start
    while i < len(text):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                break
        i += 1
    body = text[body_start + 1 : i]
    names: set[str] = set()
    depth = 0
    pending_attrs = ""
    for line in body.splitlines():
        stripped = line.strip()
        if depth == 0:
            if stripped.startswith("#["):
                pending_attrs += stripped
            m = re.match(r"^([A-Z][A-Za-z0-9]*)\s*(?:\{|\(|,|$)", stripped)
            if m:
                rn = re.search(r'name\s*=\s*"([^"]+)"', pending_attrs)
                if rn:
                    names.add(rn.group(1))
                else:
                    names.add(re.sub(r"(?<!^)(?=[A-Z])", "-", m.group(1)).lower())
                pending_attrs = ""
        depth += line.count("{") + line.count("(") - line.count("}") - line.count(")")
        if depth < 0:
            depth = 0
    return names


# ---------------------------------------------------------------------------
# Markdown extraction
# ---------------------------------------------------------------------------

FENCE_RE = re.compile(r"^(\s*)(```+|~~~+)\s*([^\s`]*)(.*)$")


def iter_blocks(md: Path):
    """Yield (lang, info, [(lineno, text), ...]) for each fenced block."""
    lines = md.read_text(encoding="utf-8").splitlines()
    i = 0
    while i < len(lines):
        m = FENCE_RE.match(lines[i])
        if not m:
            i += 1
            continue
        fence = m.group(2)
        lang = m.group(3).split(",")[0].lower()
        info = (m.group(3) + m.group(4)).lower()
        body = []
        j = i + 1
        while j < len(lines):
            if lines[j].strip().startswith(fence[0] * len(fence)) and lines[j].strip().strip(fence[0]) == "":
                break
            body.append((j + 1, lines[j]))
            j += 1
        yield lang, info, body
        i = j + 1


def logical_lines(body):
    """Join backslash continuations; yield (lineno, text)."""
    buf, start = "", None
    for lineno, text in body:
        if start is None:
            start = lineno
        stripped = text.rstrip()
        if stripped.endswith("\\") and not stripped.endswith("\\\\"):
            buf += stripped[:-1] + " "
            continue
        if stripped.endswith("`") and stripped[:-1].endswith(" "):  # powershell continuation
            buf += stripped[:-1] + " "
            continue
        buf += stripped
        yield start, buf
        buf, start = "", None
    if buf:
        yield start, buf


ENV_ASSIGN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
LAUNCHERS = {"sudo", "exec", "time", "nohup", "env", "npx", "watch"}


def strip_comment(line: str) -> str:
    """Drop a trailing shell comment (an unquoted `#` at a word start)."""
    quote = None
    for i, ch in enumerate(line):
        if quote:
            if ch == quote:
                quote = None
        elif ch in "'\"":
            quote = ch
        elif ch == "#" and (i == 0 or line[i - 1].isspace()):
            return line[:i]
    return line


def split_segments(line: str) -> list[str]:
    line = strip_comment(line)
    # Drop $(...) / backtick substitution wrappers so nested calls are seen.
    line = line.replace("$(", " ; ").replace("`", " ; ")
    return re.split(r"\s*(?:&&|\|\||\||;|&(?!\S))\s*", line)


def tokenize(seg: str) -> list[str]:
    try:
        lex = shlex.shlex(seg, posix=True, punctuation_chars=False)
        lex.whitespace_split = True
        lex.commenters = "#"
        return list(lex)
    except ValueError:
        return seg.split()


def extract_invocations(seg: str, known: set[str]) -> tuple[str, list[str]] | None:
    """Return (binary, argv) for a call to a snapshotted MockForge binary."""
    toks = tokenize(seg)
    while toks and (ENV_ASSIGN.match(toks[0]) or toks[0] in LAUNCHERS or toks[0] in {"$", ">", "PS>", "%"}):
        toks = toks[1:]
    if not toks:
        return None
    head = toks[0]
    base = head.rsplit("/", 1)[-1].rsplit("\\", 1)[-1]
    base = base.removesuffix(".exe")
    if base in known:
        return base, toks[1:]
    if head == "cargo" and "run" in toks[:3] and "--" in toks:
        pre = toks[: toks.index("--")]
        if any(t in {"mockforge-cli", "mockforge"} for t in pre):
            return "mockforge", toks[toks.index("--") + 1 :]
    return None


# Prose that names a command in order to say it does not exist.
NEGATION_BEFORE = re.compile(r"\b(?:no|not|without|instead of)\s+(?:\w+\s+)?$", re.I)

PLACEHOLDER = re.compile(r"^(<.*>|\[.*\]|\{.*\}|\$\{?\w+\}?.*|\.\.\.|…|\*)$")


def _is_value(argv: list[str], i: int) -> bool:
    return i < len(argv) and not (argv[i].startswith("-") and len(argv[i]) > 1 and not re.match(r"^-\d", argv[i]))


REDIRECT = re.compile(r"^\d*(>>?|<)(&\d+)?")


def strip_redirects(argv: list[str]) -> list[str]:
    out, skip = [], False
    for t in argv:
        if skip:
            skip = False
            continue
        m = REDIRECT.match(t)
        if m:
            skip = m.end() == len(t) and not m.group(2)  # `> file` form
            continue
        out.append(t)
    return out


def check_invocation(binary: str, argv: list[str], root: dict) -> list[str]:
    errs: list[str] = []
    argv = strip_redirects(argv)
    node = root
    trail: list[str] = []
    positionals = 0
    inherited: dict[str, bool] = dict(root["flags"])
    i = 0
    while i < len(argv):
        t = argv[i]
        if t == "--":
            break
        if t.startswith("[") and t not in {"[]"}:
            # Optional-syntax notation like `[--tags <TAGS>]`: skip to the close.
            while i < len(argv) and not argv[i].endswith("]"):
                i += 1
            i += 1
            continue
        if t.startswith("--") and len(t) > 2:
            name = t.split("=", 1)[0]
            if PLACEHOLDER.match(name[2:]) or name in {"--[options]", "--..."}:
                i += 1
                continue
            flags = {**inherited, **node["flags"]}
            if name not in flags:
                errs.append(f"unknown flag `{name}` for `{' '.join([binary, *trail])}`")
                i += 2 if "=" not in t and _is_value(argv, i + 1) else 1
                continue
            if "=" not in t and (flags[name] == 1 or (flags[name] == 2 and _is_value(argv, i + 1))):
                i += 1
            i += 1
            continue
        if t.startswith("-") and len(t) > 1 and not re.match(r"^-\d", t):
            name = t[:2]
            flags = {**inherited, **node["flags"]}
            if name not in flags:
                errs.append(f"unknown flag `{name}` for `{' '.join([binary, *trail])}`")
                i += 2 if len(t) == 2 and _is_value(argv, i + 1) else 1
                continue
            if len(t) == 2 and (flags[name] == 1 or (flags[name] == 2 and _is_value(argv, i + 1))):
                i += 1
            i += 1
            continue
        # positional
        if node.get("free"):
            break
        subs = node["subcommands"]
        if subs:
            target = t if t in subs else node["aliases"].get(t)
            if target:
                trail.append(target)
                node = subs[target]
                i += 1
                continue
            if PLACEHOLDER.match(t):
                break
            errs.append(f"unknown subcommand `{t}` under `{' '.join([binary, *trail])}`")
            break
        if PLACEHOLDER.match(t):
            break
        positionals += 1
        if "positionals" in node and positionals > node["positionals"] and not node.get("variadic"):
            errs.append(f"unexpected argument `{t}` for `{' '.join([binary, *trail])}`")
            break
        i += 1
    return errs


# ---------------------------------------------------------------------------
# HTTP routes
# ---------------------------------------------------------------------------

ROUTE_CALL_RE = re.compile(r"\.(route|route_service|nest|nest_service|merge)\(\s*")
FN_RE = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>{]*>)?\s*\(")
IDENT_CALL_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)\s*(?:::<[^>]*>)?\s*\(")
LET_RE_T = r"let\s+(?:mut\s+)?{name}\s*(?::[^=;]*)?=\s*([^;]*);"
PARAM_SEG = re.compile(r"^(\{[^}]*\}|:[A-Za-z_][A-Za-z0-9_]*|<[^>]*>|\*[A-Za-z_]*)$")


def strip_test_modules(text: str) -> str:
    idx = text.find("#[cfg(test)]")
    return text if idx < 0 else text[:idx]


def _balanced(text: str, i: int, open_c: str = "(", close_c: str = ")") -> int:
    """Return index of the char closing the bracket opened just before i."""
    depth = 1
    in_str = False
    while i < len(text):
        ch = text[i]
        if in_str:
            if ch == "\\":
                i += 2
                continue
            if ch == '"':
                in_str = False
        elif ch == '"':
            in_str = True
        elif ch == open_c:
            depth += 1
        elif ch == close_c:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return len(text)


def _functions(text: str):
    """Yield (name, body) for every fn with a body in text."""
    for m in FN_RE.finditer(text):
        close = _balanced(text, m.end())
        j = close + 1
        while j < len(text) and text[j] not in "{;":
            j += 1
        if j >= len(text) or text[j] == ";":
            continue
        body_end = _balanced(text, j + 1, "{", "}")
        yield m.group(1), text[j + 1 : body_end]


def collect_routes() -> list[str]:
    """Resolve full route paths by following `.nest`/`.merge` edges between
    router-building functions. Returns concrete path patterns."""
    local: dict[str, set[str]] = defaultdict(set)
    edges: dict[str, set[tuple[str, str]]] = defaultdict(set)  # fn -> {(prefix, target_fn)}
    unresolved: list[tuple[str, str]] = []  # (fn, prefix)
    for rs in (REPO / "crates").glob("*/src/**/*.rs"):
        if "/tests/" in str(rs) or rs.name == "tests.rs":
            continue
        text = strip_test_modules(rs.read_text(encoding="utf-8", errors="replace"))
        text = "\n".join(l for l in text.splitlines() if not l.lstrip().startswith("//"))
        for fname, body in _functions(text):
            for m in ROUTE_CALL_RE.finditer(body):
                kind = m.group(1)
                close = _balanced(body, m.end())
                args = body[m.end() : close]
                lit = re.match(r'(?:&?format!\(\s*)?"([^"]*)"\s*\)?\s*(?:,\s*(.*))?$', args, re.S)
                if kind in {"route", "route_service"}:
                    if lit and lit.group(1).startswith("/"):
                        local[fname].add(lit.group(1))
                    continue
                if kind == "merge":
                    prefix, expr = "", args
                else:
                    if not lit or not lit.group(1).startswith("/"):
                        continue
                    prefix, expr = lit.group(1).rstrip("/"), lit.group(2) or ""
                targets = [t for t in IDENT_CALL_RE.findall(expr) if t not in {"Arc", "clone", "with_state", "new", "Router", "Some", "into"}]
                if not targets:
                    var = re.match(r"\s*([A-Za-z_][A-Za-z0-9_]*)", expr)
                    if var:
                        lm = None
                        for lm in re.finditer(LET_RE_T.format(name=re.escape(var.group(1))), body, re.S):
                            pass
                        if lm:
                            targets = [t for t in IDENT_CALL_RE.findall(lm.group(1)) if t not in {"Arc", "clone", "with_state", "new", "Router", "Some", "into"}]
                if targets:
                    for t in targets:
                        edges[fname].add((prefix, t))
                elif prefix:
                    unresolved.append((fname, prefix))

    memo: dict[str, set[str]] = {}

    def full(fn: str, stack: tuple = ()) -> set[str]:
        if fn in memo:
            return memo[fn]
        if fn in stack:
            return set()
        out = set(local.get(fn, ()))
        for prefix, tgt in edges.get(fn, ()):
            for r in full(tgt, stack + (fn,)):
                out.add(prefix + (r if r != "/" else "") or "/")
        memo[fn] = out
        return out

    allp: set[str] = set()
    for fn in set(local) | set(edges):
        allp |= full(fn)
    # Unresolved nests (router passed through an opaque variable): accept the
    # prefix itself plus any route from routers that are nested nowhere else.
    for _fn, prefix in unresolved:
        allp.add(prefix)
        allp.add(prefix + "/*rest")
    return sorted(p for p in allp if norm_segments(p) and not PARAM_SEG.match(norm_segments(p)[0]))


def norm_segments(p: str) -> list[str]:
    p = p.split("?", 1)[0].split("#", 1)[0].rstrip("/") or "/"
    return [s for s in p.split("/") if s != ""]


def path_matches(doc: list[str], route: list[str]) -> bool:
    if doc and doc[-1].endswith("*"):
        # Wildcard claim (`/api/chaos/*`, `/__mockforge/**`, `persona*`):
        # some route must live under that prefix.
        head, stem = doc[:-1], doc[-1].rstrip("*")
        if len(route) < len(head) + (1 if stem else 0):
            return False
        for d, r in zip(head, route):
            if r != d and not PARAM_SEG.match(r):
                return False
        return not stem or route[len(head)].startswith(stem) or bool(PARAM_SEG.match(route[len(head)]))
    if route and route[-1].startswith("*"):
        return len(doc) >= len(route) - 1 and all(
            PARAM_SEG.match(r) or r == d for r, d in zip(route[:-1], doc)
        )
    if len(doc) != len(route):
        return False
    for d, r in zip(doc, route):
        if r == d:
            continue
        if PARAM_SEG.match(r):
            continue
        if PARAM_SEG.match(d) and PARAM_SEG.match(r):
            continue
        return False
    return True


DOC_PATH_RE = re.compile(r"(?<![A-Za-z0-9_.-])(/(?:__mockforge|api)(?:/[A-Za-z0-9_.{}:<>*-]+)*)/?")


def owned_api_prefixes(index: list[list[str]]) -> set[tuple[str, ...]]:
    owned: set[tuple[str, ...]] = set()
    for segs in index:
        if len(segs) >= 2 and segs[0] == "api":
            if re.match(r"^v\d+$", segs[1]):
                if len(segs) >= 3 and not PARAM_SEG.match(segs[2]):
                    owned.add(tuple(segs[:3]))
            elif not PARAM_SEG.match(segs[1]):
                owned.add(tuple(segs[:2]))
    return owned


# Namespaces that docs use for user-defined mock APIs; never treat these as
# MockForge's own surface even if a MockForge crate happens to register them.
USER_API_NAMESPACES = {
    ("api", "users"), ("api", "orders"), ("api", "products"), ("api", "payments"),
    ("api", "admin"), ("api", "internal"), ("api", "reports"),
    ("api", "v1", "users"), ("api", "v1", "payments"), ("api", "v1", "orders"),
}


def http_path_claims(text: str):
    for m in DOC_PATH_RE.finditer(text):
        p = m.group(1)
        p = p.rstrip(".:,)")
        yield p


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument(
        "--refresh-cli",
        metavar="BINARY",
        nargs="+",
        help="regenerate the CLI snapshot from built binaries (mockforge, mockforge-plugin, mockforge-tui)",
    )
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument("--book", type=Path, default=BOOK, help="book source dir (default: book/src)")
    args = ap.parse_args()

    if args.refresh_cli:
        refresh_snapshot(args.refresh_cli)

    snap = json.loads(SNAPSHOT.read_text())
    root = snap["mockforge"]["root"]
    known = set(snap)

    problems: list[str] = []

    # Staleness guard: snapshot vs Commands enum.
    code_cmds = code_top_level_commands()
    snap_cmds = set(root["subcommands"]) - {"help"}
    if code_cmds:
        missing = sorted(code_cmds - snap_cmds)
        extra = sorted(snap_cmds - code_cmds)
        if missing or extra:
            print(
                "CLI snapshot is stale relative to crates/mockforge-cli/src/main.rs:\n"
                f"  in code, not in snapshot: {missing}\n  in snapshot, not in code: {extra}\n"
                "Rebuild the CLI and rerun with --refresh-cli (see module docstring)."
            )
            return 1

    index = [norm_segments(p) for p in collect_routes()]
    owned = owned_api_prefixes(index) - USER_API_NAMESPACES

    per_page: dict[str, int] = defaultdict(int)
    n_cmds = n_paths = 0
    book = args.book.resolve()
    for md in sorted(book.rglob("*.md")):
        rel = Path("book/src") / md.relative_to(book)
        if rel.as_posix() in SKIP_PAGES:
            continue
        for lang, info, body in iter_blocks(md):
            if "ignore-docs-check" in info:
                continue
            if lang in SHELL_LANGS:
                for lineno, line in logical_lines(body):
                    if "docs-check: ignore" in line:
                        continue
                    for seg in split_segments(line):
                        hit = extract_invocations(seg, known)
                        if hit is None:
                            continue
                        binary, argv = hit
                        n_cmds += 1
                        for err in check_invocation(binary, argv, snap[binary]["root"]):
                            problems.append(f"{rel}:{lineno}: {err}")
                            per_page[str(rel)] += 1
        in_fence = False
        for lineno, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
            if FENCE_RE.match(line):
                in_fence = not in_fence
            if "docs-check: ignore" in line:
                continue
            if not in_fence:
                # Inline code spans in prose and tables: `mockforge foo --bar`
                for sm in re.finditer(r"`([^`]+)`", line):
                    span = sm.group(1)
                    if NEGATION_BEFORE.search(line[: sm.start()]):
                        continue  # "there is no `mockforge foo` command"
                    hit = extract_invocations(span, known)
                    if hit is None:
                        continue
                    binary, argv = hit
                    n_cmds += 1
                    for err in check_invocation(binary, argv, snap[binary]["root"]):
                        problems.append(f"{rel}:{lineno}: {err}")
                        per_page[str(rel)] += 1
            for p in http_path_claims(line):
                segs = norm_segments(p)
                if segs and segs[0] == "api":
                    key3, key2 = tuple(segs[:3]), tuple(segs[:2])
                    if key3 not in owned and key2 not in owned:
                        continue
                if segs == ["__mockforge"] or segs == ["api"]:
                    continue
                if segs and segs[0] == "api" and tuple(segs) in owned:
                    continue  # bare namespace mention, e.g. "`/api/chaos`"
                n_paths += 1
                if not any(path_matches(segs, r) for r in index):
                    problems.append(f"{rel}:{lineno}: unknown HTTP path `{p}`")
                    per_page[str(rel)] += 1

    print(
        f"checked {n_cmds} mockforge invocations and {n_paths} HTTP paths "
        f"against CLI snapshot {snap['mockforge']['version']} and {len(index)} route patterns"
    )
    for p in problems:
        print(p)
    if args.verbose:
        for page, n in sorted(per_page.items(), key=lambda kv: -kv[1]):
            print(f"{n:5d}  {page}")
    print(f"misses: {len(problems)} (baseline {BASELINE})")
    return 1 if len(problems) > BASELINE else 0


if __name__ == "__main__":
    sys.exit(main())
