#!/usr/bin/env bash
# End-to-end check that k6 behind the smokescreen egress proxy (Dockerfile.egress)
# reaches public targets but not internal ones, with the same environment the
# runner gives k6 (mockforge_bench::executor::k6_egress_env).
#
# Topology: k6 sits on an --internal network with no route out, so any public
# success proves k6 went through the proxy, for http and https (CONNECT). A
# "victim" HTTP server shares that network with the proxy, so the proxy could
# route to it; a zero hit count proves the IP policy, not routing, stopped it.
#
#   EGRESS_IMAGE=mockforge-egress:verify bash scripts/tests/k6_egress_e2e.sh
#
# Needs docker and public internet. DENY_RANGES must stay in step with the
# mockforge-egress service in saas-platform's apps-compose.yml.
set -euo pipefail

EGRESS_IMAGE="${EGRESS_IMAGE:-mockforge-egress:verify}"
K6_IMAGE="${K6_IMAGE:-grafana/k6:2.3.0}"
VICTIM_IMAGE="${VICTIM_IMAGE:-busybox:1.37}"
DENY_RANGES=(127.0.0.0/8 10.0.0.0/8 172.16.0.0/12 192.168.0.0/16 169.254.0.0/16
  100.64.0.0/10 198.18.0.0/15 0.0.0.0/8 ::1/128 fc00::/7 fe80::/10 64:ff9b::/96)

id="k6egress$$"
fail=0
cleanup() {
  docker rm -f "$id-egress" "$id-victim" >/dev/null 2>&1 || true
  docker network rm "$id-in" "$id-out" >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
tmp="$(mktemp -d)"
trap cleanup EXIT

cat >"$tmp/probe.js" <<'JS'
import http from 'k6/http';
export const options = { vus: 1, iterations: 1 };
export default function () {
  const r = http.get(__ENV.URL, { timeout: '15s' });
  console.log(`RESULT status=${r.status} error=${r.error_code}`);
}
JS
chmod 0644 "$tmp/probe.js"

docker network create --internal "$id-in" >/dev/null
docker network create "$id-out" >/dev/null
docker run -d --name "$id-victim" --network "$id-in" "$VICTIM_IMAGE" \
  sh -c 'mkdir -p /www && echo victim > /www/index.html && httpd -f -v -p 8080 -h /www' >/dev/null
deny=()
for r in "${DENY_RANGES[@]}"; do deny+=("--deny-range=$r"); done
docker run -d --name "$id-egress" --network "$id-out" "$EGRESS_IMAGE" \
  --listen-ip=0.0.0.0 --listen-port=4750 "${deny[@]}" >/dev/null
docker network connect "$id-in" "$id-egress"
victim_ip="$(docker inspect -f "{{(index .NetworkSettings.Networks \"$id-in\").IPAddress}}" "$id-victim")"
sleep 2

# k6 <url> [proxy|direct] -> prints "status=<n> error=<code>"
k6() {
  local env=(-e "URL=$1")
  if [[ "${2:-proxy}" == proxy ]]; then
    local p="http://$id-egress:4750"
    env+=(-e "HTTP_PROXY=$p" -e "HTTPS_PROXY=$p" -e "http_proxy=$p" -e "https_proxy=$p"
      -e NO_PROXY= -e no_proxy= -e K6_MAX_REDIRECTS=0 -e K6_NO_USAGE_REPORT=true)
  fi
  docker run --rm --network "$id-in" "${env[@]}" -v "$tmp/probe.js:/probe.js:ro" \
    "$K6_IMAGE" run -q /probe.js 2>&1 | grep -o 'RESULT status=[0-9]* error=[0-9]*' \
    | sed 's/RESULT //' || echo "status=none"
}
victim_hits() { docker logs "$id-victim" 2>&1 | grep -c 'GET /' || true; }

expect() { # name, actual, pattern
  if [[ "$2" =~ $3 ]]; then echo "ok   $1: $2"; else echo "FAIL $1: $2 (want $3)"; fail=1; fi
}

expect "https public via proxy (CONNECT)" "$(k6 https://registry.mockforge.dev/health)" '^status=200 '
expect "http public via proxy" "$(k6 http://example.com/)" '^status=200 '
expect "control: victim reachable without the proxy" "$(k6 "http://$victim_ip:8080/" direct)" '^status=200 '
before="$(victim_hits)"
expect "internal IP literal via proxy" "$(k6 "http://$victim_ip:8080/")" '^status=(0|403|407|502|503) '
expect "internal Docker name via proxy" "$(k6 "http://$id-victim:8080/")" '^status=(0|403|407|502|503) '
expect "public name resolving to the internal IP" "$(k6 "http://$victim_ip.nip.io:8080/")" '^status=(0|403|407|502|503) '
expect "public name resolving to loopback" "$(k6 http://127.0.0.1.nip.io:4750/)" '^status=(0|403|407|502|503) '
expect "https to an internal name (CONNECT)" "$(k6 "https://$victim_ip.nip.io:8080/")" '^status=(0|403|407|502|503) '
expect "redirects are not followed" "$(k6 "http://httpbin.org/redirect-to?url=http%3A%2F%2F$victim_ip%3A8080%2F")" '^status=302 '
after="$(victim_hits)"
expect "victim saw no proxied request" "$((after - before))" '^0$'
# The proxy itself must have made the five denials, after resolving names.
denied="$(docker logs "$id-egress" 2>&1 | grep 'CANONICAL-PROXY-DECISION' | grep -c '"allow":false' || true)"
expect "smokescreen logged the denials" "$denied" '^[5-9]$'

exit "$fail"
