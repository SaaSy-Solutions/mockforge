#!/usr/bin/env python3
"""Qualify and measure one auth-edit -> TCP-check invalidation boundary."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time

BASE = Path('/mnt/build/worktrees')
TREES = {name: BASE / f'mf-protocol-bench-{name}-1477' for name in ('before', 'after')}
COMMITS = {'before': '3871677f241ee8e9726ad7646fa356c0a2df04ca',
           'after': '56dc1ffa2dabdcaf67d914cf6dc85ce41f9b7166'}
AUTH = Path('crates/mockforge-core/src/protocol_abstraction/auth.rs')
NEEDLE = b'name: "AuthMiddleware".to_string(),'
COMMAND = ['scripts/dev/cargo', 'check', '--locked', '-p', 'mockforge-tcp',
           '--lib', '--message-format=json']
OUTPUT = Path('/tmp/mockforge-protocol1477-probe')
MIN_FREE = 60 * 1024**3
OVERRIDES = ('RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'RUSTFLAGS',
             'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_JOBS', 'CARGO_TARGET_DIR',
             'CARGO_BUILD_TARGET_DIR', 'RUSTUP_TOOLCHAIN', 'CARGO_PROFILE_DEV_INCREMENTAL',
             'CARGO_INCREMENTAL')


def git(tree, *args):
    return subprocess.check_output(['git', *args], cwd=tree, text=True).strip()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def host():
    return {'time_ns': time.time_ns(), 'loadavg': Path('/proc/loadavg').read_text().strip(),
            'meminfo': Path('/proc/meminfo').read_text(),
            'stat': Path('/proc/stat').read_text().splitlines()[0],
            'pressure': {name: Path(f'/proc/pressure/{name}').read_text()
                         for name in ('cpu', 'io', 'memory')},
            'free_bytes': shutil.disk_usage(BASE).free}


def guard_disk():
    free = shutil.disk_usage(BASE).free
    if free < MIN_FREE:
        raise RuntimeError(f'Disk floor: {free} bytes available, require {MIN_FREE}')


def setup():
    OUTPUT.mkdir(exist_ok=True)
    for name in OVERRIDES:
        if os.environ.get(name):
            raise RuntimeError(f'Refusing ambient override: {name}')
    originals = {}
    identities = {}
    for side, tree in TREES.items():
        if git(tree, 'rev-parse', 'HEAD') != COMMITS[side] or git(tree, 'status', '--porcelain'):
            raise RuntimeError(f'Require clean pinned worktree: {tree}')
        originals[side] = (tree / AUTH).read_bytes()
        version = subprocess.check_output(['rustc', '-Vv'], cwd=tree, text=True)
        if not version.startswith('rustc 1.96.0 '):
            raise RuntimeError(f'Unexpected toolchain in {side}: {version}')
        paths = ['rust-toolchain.toml', '.cargo/config.toml', 'scripts/dev/cargo',
                 'scripts/dev/rust_build_policy.py', 'scripts/dev/rust_policy_config.py']
        identities[side] = {'commit': COMMITS[side], 'tree': str(tree),
                            'target': str(tree / 'target'), 'rustc': version,
                            'auth_sha256': digest(originals[side]),
                            'config_sha256': {p: digest((tree / p).read_bytes()) for p in paths},
                            'lock_sha256': digest((tree / 'Cargo.lock').read_bytes())}
    if originals['before'] != originals['after']:
        raise RuntimeError('Auth source differs between endpoints')
    if identities['before']['config_sha256'] != identities['after']['config_sha256']:
        raise RuntimeError('Wrapper/toolchain/config differs between endpoints')
    (OUTPUT / 'identities.json').write_text(json.dumps(identities, indent=2) + '\n')
    return originals


def profile_sample(tree):
    """Sample only this checkout's Cargo/rustc and bounded wrapper scope."""
    result = []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():
            continue
        try:
            command = (proc / 'cmdline').read_bytes().split(b'\0')
            is_wrapper = (str(tree / 'scripts/dev/cargo').encode() in command
                          and b'--inside-scope' in command)
            if not is_wrapper:
                if not Path(os.readlink(proc / 'cwd')).is_relative_to(tree):
                    continue
                executable = command[0].split(b'/')[-1]
                if executable not in (b'rustc', b'cargo'):
                    continue
            stat = (proc / 'stat').read_text().rsplit(')', 1)[1].split()
            entry = {'pid': int(proc.name), 'ticks': int(stat[11]) + int(stat[12]),
                     'kind': 'wrapper' if is_wrapper else command[0].decode()}
            if b'--crate-name' in command:
                entry['crate'] = command[command.index(b'--crate-name') + 1].decode()
            group = (proc / 'cgroup').read_text().split('0::', 1)[1].strip()
            cg = Path('/sys/fs/cgroup') / group.lstrip('/')
            entry['cgroup'] = group
            for key, filename in [('cpu', 'cpu.stat'), ('io', 'io.stat'), ('memory', 'memory.current')]:
                try:
                    entry[key] = (cg / filename).read_text().strip()
                except OSError as error:
                    entry[key + '_error'] = type(error).__name__
            result.append(entry)
        except (OSError, ValueError, IndexError):
            continue
    return result


def run(side, label, profile=False):
    guard_disk()
    tree = TREES[side]
    prefix = OUTPUT / f'{label}-{side}'
    before = host()
    started = time.monotonic()
    samples = []
    with prefix.with_suffix('.cargo.jsonl').open('w') as stdout, \
            prefix.with_suffix('.stderr').open('w') as stderr:
        process = subprocess.Popen(COMMAND, cwd=tree, stdout=stdout, stderr=stderr)
        try:
            while process.poll() is None:
                if profile:
                    samples.append({'elapsed': time.monotonic() - started,
                                    'processes': profile_sample(tree)})
                time.sleep(0.15 if profile else 0.1)
        finally:
            if process.poll() is None:
                process.send_signal(signal.SIGINT)
                process.wait(timeout=60)
    wall = time.monotonic() - started
    after = host()
    messages = [json.loads(line) for line in prefix.with_suffix('.cargo.jsonl').read_text().splitlines()
                if line.startswith('{')]
    events = []
    for line in prefix.with_suffix('.stderr').read_text().splitlines():
        if line.startswith('{'):
            item = json.loads(line)
            if item.get('cargo_command') == 'check':
                events.append(item)
    if process.returncode or not any(m.get('reason') == 'build-finished' and m.get('success')
                                     for m in messages) or len(events) != 1:
        raise RuntimeError(f'Build/wrapper failed: {prefix}')
    event = events[0]
    if event['exit_status'] != 0:
        raise RuntimeError(f'Wrapper failure: {event}')
    artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact']
    names = {m['target']['name']: m['fresh'] for m in artifacts}
    row = {'side': side, 'label': label, 'wall_seconds': wall,
           'wrapper': event, 'before': before, 'after': after,
           'artifacts': [{'package_id': m['package_id'], 'target': m['target'],
                          'fresh': m['fresh'], 'features': m['features']}
                         for m in artifacts]}
    prefix.with_suffix('.result.json').write_text(json.dumps(row, indent=2) + '\n')
    if profile:
        prefix.with_suffix('.profile.json').write_text(json.dumps(samples, indent=2) + '\n')
    print(json.dumps({'side': side, 'label': label, 'wall': round(wall, 3),
                      'execution': event['execution_seconds'],
                      'admission': event['admission_seconds'],
                      'rebuilt': [name for name, fresh in names.items() if not fresh]}), flush=True)
    return names, row


def verify(names, side, changed):
    if names.get('mockforge_tcp') is not (not changed or side == 'after'):
        raise RuntimeError(f'TCP freshness mismatch: {side}: {names.get("mockforge_tcp")}')
    if side == 'after' and 'mockforge_core' in names:
        raise RuntimeError('Core remains in after TCP check graph')
    if side == 'before' and names.get('mockforge_core') is not (not changed):
        raise RuntimeError(f'Core freshness mismatch: {names.get("mockforge_core")}')
    rebuilt = {name for name, fresh in names.items() if not fresh}
    expected = {'mockforge_core', 'mockforge_tcp'} if changed and side == 'before' else set()
    if rebuilt != expected:
        raise RuntimeError(f'Unexpected rebuild set: {rebuilt}, expected {expected}')


def perturb(side, label, originals, profile=False):
    tree = TREES[side]
    original = originals[side]
    if original.count(NEEDLE) != 1:
        raise RuntimeError('Expected exactly one auth name initializer')
    probe = b'name: "AuthMiddlewareBenchmark' + label.encode() + b'".to_string(),'
    try:
        (tree / AUTH).write_bytes(original.replace(NEEDLE, probe))
        names, row = run(side, label, profile)
        verify(names, side, changed=True)
        return row
    finally:
        (tree / AUTH).write_bytes(original)
        if (tree / AUTH).read_bytes() != original or git(tree, 'status', '--porcelain'):
            raise RuntimeError(f'Failed to restore clean source: {tree}')



def require_profiled_core_cpu(samples):
    """Fail closed unless the owned Core process accumulated sampled CPU."""
    ticks = {}
    for sample in samples:
        for process in sample['processes']:
            if process.get('crate') == 'mockforge_core':
                ticks.setdefault(process['pid'], []).append(process['ticks'])
    delta = sum(max(values) - min(values) for values in ticks.values())
    if delta <= 0:
        raise RuntimeError('Profiling requires positive owned Core CPU delta')
    rate = os.sysconf('SC_CLK_TCK')
    return {'sampled_cpu_delta_seconds': delta / rate,
            'max_observed_cpu_seconds': sum(max(values) for values in ticks.values()) / rate}


def test_profile_guard():
    for invalid in ([], [{'processes': [{'pid': 1, 'crate': 'mockforge_tcp', 'ticks': 10}]}],
                    [{'processes': [{'pid': 1, 'crate': 'mockforge_core', 'ticks': 0}]},
                     {'processes': [{'pid': 1, 'crate': 'mockforge_core', 'ticks': 0}]}]):
        try:
            require_profiled_core_cpu(invalid)
        except RuntimeError:
            pass
        else:
            raise AssertionError('Accepted empty, missing or zero-CPU Core profile')
    positive = [{'processes': [{'pid': 1, 'crate': 'mockforge_core', 'ticks': value}]}
                for value in (10, 30)]
    assert require_profiled_core_cpu(positive)['sampled_cpu_delta_seconds'] > 0
    print('Profile guard: empty/no-Core/zero-Core rejected; positive Core accepted')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['qualify', 'profile', 'trials', 'self-test'])
    args = parser.parse_args()
    if args.phase == 'self-test':
        test_profile_guard()
        return
    originals = setup()
    try:
        if args.phase in ('qualify', 'profile'):
            if args.phase == 'qualify':
                for side in TREES:
                    run(side, 'warm')
                    names, _ = run(side, 'fresh')
                    verify(names, side, changed=False)
            pilot = perturb('before', 'ProfilePilot2', originals, profile=True)
            profile = json.loads((OUTPUT / 'ProfilePilot2-before.profile.json').read_text())
            core_cpu = require_profiled_core_cpu(profile)
            # Restoring original bytes changes the input again; rewarm explicitly.
            run('before', 'rewarm')
            names, _ = run('before', 'refreshed')
            verify(names, 'before', changed=False)
            (OUTPUT / 'qualified.json').write_text(json.dumps({
                'script_sha256': digest(Path(__file__).read_bytes()),
                'pilot_execution_seconds': pilot['wrapper']['execution_seconds'],
                'core_cpu': core_cpu}, indent=2) + '\n')
        else:
            qualification = json.loads((OUTPUT / 'qualified.json').read_text())
            if qualification['script_sha256'] != digest(Path(__file__).read_bytes()):
                raise RuntimeError('Probe changed since qualification')
            rows = []
            for pair in range(1, 6):
                for side in (['before', 'after'] if pair % 2 else ['after', 'before']):
                    rows.append(perturb(side, f'Pair{pair}', originals))
            fingerprints = {row['wrapper']['policy_sha256'] for row in rows}
            limits = {json.dumps(row['wrapper']['limits'], sort_keys=True) for row in rows}
            if len(fingerprints) != 1 or len(limits) != 1:
                raise RuntimeError('Runtime policy or resource limits changed across trials')
            (OUTPUT / 'trials.json').write_text(json.dumps(rows, indent=2) + '\n')
    finally:
        for side, tree in TREES.items():
            (tree / AUTH).write_bytes(originals[side])
            if git(tree, 'status', '--porcelain'):
                raise RuntimeError(f'Worktree not clean: {tree}')


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, lambda signum, frame: (_ for _ in ()).throw(KeyboardInterrupt()))
    main()
