# Running `mockforge serve` unattended

Two equivalent options, pick one:

| Option | When to use | File |
|---|---|---|
| **systemd service** | Linux host, want logs in `journalctl`, OS-level restart policy, resource limits | [`mockforge.service`](mockforge.service) |
| **`run-forever.sh` wrapper** | macOS / non-systemd host, container, ad-hoc test session, or bench rigs | [`../scripts/run-forever.sh`](../scripts/run-forever.sh) |

Both restart `mockforge serve` after any non-clean exit. Issue #79 — heavy
chunked-upload traffic can OOM-kill or crash the server; either wrapper
keeps it alive without manual intervention.

## systemd quick start

```bash
sudo install -m 0755 target/release/mockforge /usr/local/bin/mockforge
sudo useradd -r -s /usr/sbin/nologin mockforge
sudo install -d -o mockforge -g mockforge /etc/mockforge /var/log/mockforge
sudo cp your-spec.yaml /etc/mockforge/spec.yaml
sudo install -m 0644 deploy/systemd/mockforge.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now mockforge

# Tail logs
journalctl -u mockforge -f

# Override flags without editing the unit file
sudo systemctl edit mockforge   # adds /etc/systemd/system/mockforge.service.d/override.conf
```

## run-forever quick start

```bash
deploy/scripts/run-forever.sh \
  --spec /path/to/spec.yaml \
  --http-port 80 --https-port 443 \
  --tls-cert server.pem --tls-key server.key \
  --no-rate-limit

# With a restart log
RESTART_LOG=/var/log/mockforge-supervisor.log \
RESTART_BACKOFF_SECS=10 \
  deploy/scripts/run-forever.sh --spec api.json --admin --admin-port 9080

# Stop: Ctrl-C, or `kill <pid>` — SIGINT/SIGTERM are forwarded to the child.
```

## Tuning for high traffic

If the kernel keeps OOM-killing the server even with these wrappers, the
two knobs that matter most:

- `MemoryMax` (systemd) / `ulimit -v` (shell) — bump above the working set
  size. A `bench-chunked` run with `--concurrency 10 --total-size-bytes
  10485760` will hold ~100 MiB of in-flight body buffers; size accordingly.
- `LimitNOFILE` (systemd) / `ulimit -n` (shell) — at high concurrency,
  each in-flight request is a file descriptor. 65536 covers most cases.

Set `MOCKFORGE_METRICS_LOG_FILE` to a path on persistent disk to capture
CPU / memory / TPS / RPS200 / CPS every 10 seconds across restarts —
useful for postmortem-ing exactly when and how the server fell over.
