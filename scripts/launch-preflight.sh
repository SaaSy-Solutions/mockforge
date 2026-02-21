#!/usr/bin/env bash
set -euo pipefail

# Launch readiness preflight for MockForge Cloud monetization stack.
# This checks local prerequisites and validates environment variable presence.

ENV_FILE="${1:-.env.launch}"

if [[ -f "${ENV_FILE}" ]]; then
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
fi

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  echo "[PASS] $1"
}

warn() {
  WARN_COUNT=$((WARN_COUNT + 1))
  echo "[WARN] $1"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  echo "[FAIL] $1"
}

check_cmd() {
  local cmd="$1"
  if command -v "${cmd}" >/dev/null 2>&1; then
    pass "Command available: ${cmd}"
  else
    fail "Missing command: ${cmd}"
  fi
}

check_env_required() {
  local var_name="$1"
  local value="${!var_name-}"
  if [[ -n "${value}" ]]; then
    pass "Env set: ${var_name}"
  else
    fail "Env missing: ${var_name}"
  fi
}

check_env_recommended() {
  local var_name="$1"
  local value="${!var_name-}"
  if [[ -n "${value}" ]]; then
    pass "Env set: ${var_name}"
  else
    warn "Env recommended: ${var_name}"
  fi
}

echo "== MockForge Launch Preflight =="
echo "Using env file: ${ENV_FILE}"
echo

echo "== Command Checks =="
check_cmd rg
check_cmd curl
check_cmd openssl
check_cmd docker
echo

echo "== Core Runtime Env (Required) =="
check_env_required DATABASE_URL
check_env_required JWT_SECRET
check_env_required APP_BASE_URL
check_env_required S3_BUCKET
check_env_required S3_REGION
echo

echo "== Revenue Env (Stripe - Required for paid plans) =="
check_env_required STRIPE_SECRET_KEY
check_env_required STRIPE_PRICE_ID_PRO
check_env_required STRIPE_PRICE_ID_TEAM
check_env_required STRIPE_WEBHOOK_SECRET
echo

echo "== Email Env =="
check_env_required EMAIL_PROVIDER
check_env_required EMAIL_FROM
check_env_required SUPPORT_EMAIL

EMAIL_PROVIDER_VALUE="${EMAIL_PROVIDER-}"
case "${EMAIL_PROVIDER_VALUE,,}" in
  postmark|brevo|sendinblue)
    check_env_required EMAIL_API_KEY
    ;;
  smtp)
    check_env_required SMTP_HOST
    check_env_recommended SMTP_PORT
    check_env_recommended SMTP_USERNAME
    check_env_recommended SMTP_PASSWORD
    ;;
  disabled|"")
    warn "Email provider disabled or unset; onboarding and billing email flows will not deliver."
    ;;
  *)
    warn "Unknown EMAIL_PROVIDER value: ${EMAIL_PROVIDER_VALUE}"
    ;;
esac
echo

echo "== Optional but Recommended =="
check_env_recommended REDIS_URL
check_env_recommended CORS_ALLOWED_ORIGINS
check_env_recommended OAUTH_GITHUB_CLIENT_ID
check_env_recommended OAUTH_GITHUB_CLIENT_SECRET
check_env_recommended OAUTH_GOOGLE_CLIENT_ID
check_env_recommended OAUTH_GOOGLE_CLIENT_SECRET
echo

echo "== Summary =="
echo "Pass: ${PASS_COUNT}"
echo "Warn: ${WARN_COUNT}"
echo "Fail: ${FAIL_COUNT}"

if [[ "${FAIL_COUNT}" -gt 0 ]]; then
  echo
  echo "Preflight failed. Fix missing required items, then rerun:"
  echo "  ./scripts/launch-preflight.sh .env.launch"
  exit 1
fi

echo
echo "Preflight checks passed for required launch items."
