---
allowed-tools: Bash, Read, Grep, Glob, Task
description: Pre-publish GO/NO-GO gate without publishing (runs the release-guardian agent)
---

# /release-check — Pre-Publish Gate

Run the release safety checks and return a single GO / NO-GO verdict. This does
NOT bump, commit, publish, or tag — it just tells you whether a publish is safe.
Use it before `scripts/publish-crates.sh`, or any time you've added a crate and
want to confirm the publish list is in sync.

For the full bump → publish → tag flow, use the `/ship-release` skill (which
calls this gate internally).

## Steps

1. Show the release version and tree state:
   ```bash
   git status --porcelain
   python3 -c "import tomllib,pathlib;print(tomllib.loads(pathlib.Path('Cargo.toml').read_text())['workspace']['package']['version'])"
   ```
2. Quick CRATES-list drift check (the #584 class). "Publishable" = workspace
   members whose `publish != false`, NOT a directory glob (a non-member dir is an
   orphan, not drift):
   ```bash
   cargo metadata --no-deps --format-version 1 \
     | python3 -c "import sys,json; [print(p['name']) for p in json.load(sys.stdin)['packages'] if p['name'].startswith('mockforge-') and p.get('publish') != []]" \
     | sort -u > /tmp/publishable.txt
   sed -n '64,120p' scripts/publish-crates.sh | grep -oE 'mockforge-[a-z0-9-]+' | sort -u > /tmp/listed.txt
   echo "Publishable members missing from publish-crates.sh:"; comm -23 /tmp/publishable.txt /tmp/listed.txt
   # orphan hygiene (warn only): on-disk crates that are NOT workspace members
   comm -23 <(ls -d crates/mockforge-*/ | sed 's#crates/##;s#/##' | sort -u) \
            <(cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json;[print(p['name']) for p in json.load(sys.stdin)['packages']]" | sort -u) \
     | sed 's/^/  orphan (not in workspace): /'
   ```
3. Dispatch the **`release-guardian`** agent (haiku) for the full gate
   (versions, smoke-test evidence, CHANGELOG + pillars, clean tree).
4. Print the agent's GO / NO-GO table verbatim.

## Rules
- This command is read-only — never edit or publish.
- A NO-GO means fix the FAIL rows before running `/ship-release` or
  `publish-crates.sh`.
