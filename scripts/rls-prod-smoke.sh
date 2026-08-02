#!/usr/bin/env bash
# Production smoke for the #832 RLS cutover.
#
# Run it BEFORE setting APP_DATABASE_URL and AFTER. Identical output means the
# activation was transparent; any diff is the blast radius.
#
# It deliberately exercises the paths where the two defects the e2e gate caught
# actually lived, rather than just hitting /health:
#
#   org creation   -> an audit_logs INSERT whose org_id differs from the org
#                     bound to the request. Under the first cut of the policies
#                     this was rejected and, because record_audit_event is
#                     fire-and-forget, silently dropped.
#   audit list     -> covered-table read with an explicit org.
#   hosted mocks   -> covered-table read with an explicit org.
#   marketplace    -> UNAUTHENTICATED, org-less read. This is where a missing
#                     nullif() in the policy turned a fail-closed empty result
#                     into a 500, and where binding the task-local instead of
#                     the caller's org fail-closed public search to zero.
#
# The marketplace call is repeated: the reverted-GUC bug only appears on a
# connection that has ALREADY served an org-bound transaction, so a single cold
# request can pass while the next one fails.
#
# Usage:  scripts/rls-prod-smoke.sh                 # against api.mockforge.dev
#         BASE=http://localhost:8080 scripts/rls-prod-smoke.sh
#
# Creates one throwaway user + org per run, identifiable by the `rlssmoke<ts>`
# username and `rls-smoke-<ts>` org slug.

set -uo pipefail

BASE="${BASE:-https://api.mockforge.dev}"
STAMP="$(date +%s)"
U="rlssmoke${STAMP}"
E="rlssmoke${STAMP}@mockforge-smoke.invalid"
P="SmokeTest!${STAMP}aB"

pass=0
fail=0
ok() {
  printf '  \033[32mPASS\033[0m %s\n' "$1"
  pass=$((pass + 1))
}
bad() {
  printf '  \033[31mFAIL\033[0m %s — %s\n' "$1" "$2"
  fail=$((fail + 1))
}

echo "== RLS prod smoke against $BASE =="

health="$(curl -sS -m 20 "$BASE/health" 2>&1)"
if grep -q '"status":"ok"' <<<"$health"; then
  ok "health: $health"
else
  bad "health" "$health"
fi

reg="$(curl -sS -m 30 -X POST "$BASE/api/v1/auth/register" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"$U\",\"email\":\"$E\",\"password\":\"$P\"}" 2>&1)"
TOKEN="$(jq -r '.access_token // .token // empty' <<<"$reg" 2>/dev/null)"
if [ -n "$TOKEN" ]; then ok "register + token"; else bad "register" "$reg"; fi

AUTH=(-H "Authorization: Bearer $TOKEN")

# Org-less read: the caller has no single org to bind.
orgs="$(curl -sS -m 20 "${AUTH[@]}" "$BASE/api/v1/organizations" 2>&1)"
ORG="$(jq -r 'if type=="array" then .[0].id else (.organizations[0].id // .data[0].id // empty) end' <<<"$orgs" 2>/dev/null)"
if [ -n "$ORG" ] && [ "$ORG" != "null" ]; then
  ok "list orgs -> $ORG"
else
  bad "list orgs" "$(head -c 300 <<<"$orgs")"
fi

# Creating a SECOND org is the audit-write shape: the request is bound to the
# personal org while the audit row belongs to the org being created.
neworg="$(curl -sS -m 30 -X POST "$BASE/api/v1/organizations" "${AUTH[@]}" \
  -H 'Content-Type: application/json' \
  -d "{\"name\":\"rls smoke $STAMP\",\"slug\":\"rls-smoke-$STAMP\"}" 2>&1)"
NEWORG="$(jq -r '.id // .organization.id // empty' <<<"$neworg" 2>/dev/null)"
if [ -n "$NEWORG" ]; then ok "create org -> $NEWORG"; else bad "create org" "$(head -c 300 <<<"$neworg")"; fi

al="$(curl -sS -m 20 -o /tmp/smoke_audit.json -w '%{http_code}' "${AUTH[@]}" \
  -H "X-Organization-Id: $NEWORG" "$BASE/api/v1/organizations/$NEWORG/audit-logs" 2>&1)"
n="$(jq -r 'if type=="array" then length elif .logs then (.logs|length) elif .data then (.data|length) else 0 end' /tmp/smoke_audit.json 2>/dev/null || echo 0)"
if [ "$al" = "200" ]; then
  ok "audit-logs http 200, $n row(s) for the new org"
  if [ "${n:-0}" -gt 0 ]; then
    ok "org-creation audit event persisted (n=$n) — not silently dropped"
  else
    bad "org-creation audit event" "0 rows: the write may have been rejected and swallowed"
  fi
else
  bad "audit-logs" "http $al $(head -c 200 /tmp/smoke_audit.json)"
fi

hm="$(curl -sS -m 20 -o /tmp/smoke_hm.json -w '%{http_code}' "${AUTH[@]}" \
  -H "X-Organization-Id: $ORG" "$BASE/api/v1/hosted-mocks" 2>&1)"
if [ "$hm" = "200" ]; then
  ok "hosted-mocks http 200"
else
  bad "hosted-mocks" "http $hm $(head -c 200 /tmp/smoke_hm.json)"
fi

ms="$(curl -sS -m 20 -o /tmp/smoke_ms.json -w '%{http_code}' -X POST \
  "$BASE/api/v1/marketplace/templates/search" \
  -H 'Content-Type: application/json' -d '{"query":"","tags":[]}' 2>&1)"
if [ "$ms" = "200" ]; then
  ok "marketplace search (anon) http 200"
else
  bad "marketplace search (anon)" "http $ms $(head -c 200 /tmp/smoke_ms.json)"
fi

for i in 1 2 3; do
  c="$(curl -sS -m 20 -o /dev/null -w '%{http_code}' -X POST \
    "$BASE/api/v1/marketplace/templates/search" \
    -H 'Content-Type: application/json' -d '{"query":"","tags":[]}' 2>&1)"
  if [ "$c" != "200" ]; then
    bad "marketplace repeat #$i (connection reuse)" "http $c"
    break
  fi
  [ "$i" = 3 ] && ok "marketplace search stable over 3 repeats (no GUC poisoning)"
done

echo
echo "== $pass passed, $fail failed =="
[ "$fail" -eq 0 ]
