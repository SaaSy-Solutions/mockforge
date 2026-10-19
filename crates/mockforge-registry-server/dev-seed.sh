#!/usr/bin/env bash
# Create a local development admin account with a random password (#1014).
#
# Versioned migrations never create accounts: they run against every database,
# production included. This script is the only seeding path and refuses to run
# unless it is explicitly enabled and pointed at a loopback database.
#
#   MOCKFORGE_DEV_SEED=1 ./dev-seed.sh
#
# The password is generated per run and printed once; it is not stored
# anywhere else. Re-running rotates it. Requires `psql` and `openssl`, and the
# `pgcrypto` extension (shipped with the official postgres image) for bcrypt.
set -euo pipefail

DATABASE_URL="${DATABASE_URL:-postgres://postgres:password@localhost:5432/mockforge_registry}"
DEV_ADMIN_EMAIL="${DEV_ADMIN_EMAIL:-dev-admin@mockforge.localhost}"
DEV_ADMIN_USERNAME="${DEV_ADMIN_USERNAME:-dev-admin}"

if [[ "${MOCKFORGE_DEV_SEED:-}" != "1" ]]; then
    echo "refusing to seed: set MOCKFORGE_DEV_SEED=1 to create a local dev admin" >&2
    exit 1
fi

# Extract the host from postgres://user:pass@host:port/db (or [v6]:port).
authority="${DATABASE_URL#*://}"
authority="${authority%%/*}"
authority="${authority##*@}"
if [[ "$authority" == \[* ]]; then
    db_host="${authority%%]*}]"
else
    db_host="${authority%%:*}"
fi
case "$db_host" in
    localhost | 127.0.0.1 | "[::1]" | "") ;;
    *)
        echo "refusing to seed: database host '$db_host' is not loopback" >&2
        exit 1
        ;;
esac

for value in "$DEV_ADMIN_EMAIL" "$DEV_ADMIN_USERNAME"; do
    if [[ ! "$value" =~ ^[A-Za-z0-9@._-]+$ ]]; then
        echo "refusing to seed: '$value' contains unsupported characters" >&2
        exit 1
    fi
done

# URL-safe alphabet only, so the value can be embedded in SQL as-is.
password="$(openssl rand -base64 24 | tr '+/' '-_' | tr -d '=\n')"

# The SQL goes over stdin so the password never appears in a process list.
psql "$DATABASE_URL" --quiet --no-psqlrc -v ON_ERROR_STOP=1 >/dev/null <<SQL
CREATE EXTENSION IF NOT EXISTS pgcrypto;
INSERT INTO users (username, email, password_hash, is_admin, is_verified)
VALUES (
    '$DEV_ADMIN_USERNAME',
    '$DEV_ADMIN_EMAIL',
    crypt('$password', gen_salt('bf', 12)),
    TRUE,
    TRUE
)
ON CONFLICT (email) DO UPDATE
SET password_hash = EXCLUDED.password_hash,
    is_admin = TRUE,
    is_verified = TRUE,
    updated_at = NOW();
SQL

echo "Local dev admin ready (this password is shown once; re-run to rotate):"
echo "  email:    $DEV_ADMIN_EMAIL"
echo "  password: $password"
