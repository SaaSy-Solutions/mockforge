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
  docker rm -f "$id-egress" "$id-victim" "$id-redirector" >/dev/null 2>&1 || true
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
ip_on_in() { docker inspect -f "{{(index .NetworkSettings.Networks \"$id-in\").IPAddress}}" "$1"; }
victim_ip="$(ip_on_in "$id-victim")"
# A redirector the test owns, instead of a third-party service (#1639): every
# request gets a 302 to the victim. The proxy may reach it only through the
# /32 allow rule below; smokescreen checks --allow-range before --deny-range
# and its private-range default, so nothing else on the network is opened.
docker run -d --name "$id-redirector" --network "$id-in" -e "TARGET=http://$victim_ip:8080/" \
  "$VICTIM_IMAGE" sh -c 'while true; do printf "HTTP/1.1 302 Found\r\nLocation: %s\r\nContent-Length: 0\r\nConnection: close\r\n\r\n" "$TARGET" | nc -l -p 8081 >/dev/null; done' >/dev/null
redirector_ip="$(ip_on_in "$id-redirector")"
deny=()
for r in "${DENY_RANGES[@]}"; do deny+=("--deny-range=$r"); done
docker run -d --name "$id-egress" --network "$id-out" "$EGRESS_IMAGE" \
  --listen-ip=0.0.0.0 --listen-port=4750 "--allow-range=$redirector_ip/32" "${deny[@]}" >/dev/null
docker network connect "$id-in" "$id-egress"
sleep 2

# k6 <url> [proxy|proxy-follow|direct] -> prints "status=<n> error=<code>"
# proxy: the runner's environment (no redirects followed). proxy-follow: the
# same proxy, but k6 follows redirects. direct: no proxy, redirects followed.
k6() {
  local env=(-e "URL=$1")
  local mode="${2:-proxy}"
  if [[ "$mode" == proxy || "$mode" == proxy-follow ]]; then
    local p="http://$id-egress:4750"
    env+=(-e "HTTP_PROXY=$p" -e "HTTPS_PROXY=$p" -e "http_proxy=$p" -e "https_proxy=$p"
      -e NO_PROXY= -e no_proxy= -e K6_NO_USAGE_REPORT=true)
    [[ "$mode" == proxy ]] && env+=(-e K6_MAX_REDIRECTS=0)
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
# Negative control for the redirect cases: without the proxy, following the
# redirector's 302 does reach the victim, so a zero hit count below means
# containment held, not that the redirect never pointed anywhere.
hits0="$(victim_hits)"
expect "control: redirect reaches the victim without the proxy" "$(k6 "http://$redirector_ip:8081/" direct)" '^status=200 '
expect "control: victim saw the redirected request" "$(( $(victim_hits) - hits0 ))" '^1$'
before="$(victim_hits)"
expect "internal IP literal via proxy" "$(k6 "http://$victim_ip:8080/")" '^status=(0|403|407|502|503) '
expect "internal Docker name via proxy" "$(k6 "http://$id-victim:8080/")" '^status=(0|403|407|502|503) '
expect "public name resolving to the internal IP" "$(k6 "http://$victim_ip.nip.io:8080/")" '^status=(0|403|407|502|503) '
expect "public name resolving to loopback" "$(k6 http://127.0.0.1.nip.io:4750/)" '^status=(0|403|407|502|503) '
expect "https to an internal name (CONNECT)" "$(k6 "https://$victim_ip.nip.io:8080/")" '^status=(0|403|407|502|503) '
expect "redirects are not followed" "$(k6 "http://$redirector_ip:8081/")" '^status=302 '
expect "a followed redirect to the victim is denied" "$(k6 "http://$redirector_ip:8081/" proxy-follow)" '^status=(0|403|407|502|503) '
after="$(victim_hits)"
expect "victim saw no proxied request" "$((after - before))" '^0$'
# The proxy itself must have made the six denials, after resolving names.
denied="$(docker logs "$id-egress" 2>&1 | grep 'CANONICAL-PROXY-DECISION' | grep -c '"allow":false' || true)"
expect "smokescreen logged the denials" "$denied" '^[6-9]$'

exit "$fail"
