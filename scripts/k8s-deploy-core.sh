#!/usr/bin/env bash
# Deploy the core MockForge manifests from k8s/ into a cluster.
#
# Used by .github/workflows/k8s-tests.yml (KinD) and runnable locally:
#
#     scripts/k8s-deploy-core.sh [image] [namespace]
#
# "Core" is what MockForge needs to run on a bare cluster. The other manifests
# in k8s/ are add-ons that depend on things a bare cluster does not have, so
# they are only validated with a server-side dry run:
#   - servicemonitor.yaml     needs the Prometheus Operator CRDs
#   - vault-integration.yaml  needs the External Secrets Operator CRDs + Vault
#   - network-policy.yaml     default-denies ingress, which would also block
#                             the out-of-namespace smoke test
#   - ingress.yaml, cdn-config.yaml, hpa.yaml, redis.yaml  need an ingress
#                             controller / metrics-server / are optional
set -euo pipefail

IMAGE="${1:-ghcr.io/saasy-solutions/mockforge:latest}"
NAMESPACE="${2:-mockforge}"
K8S_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../k8s" && pwd)"

CORE=(pod-security.yaml rbac.yaml configmap.yaml service.yaml statefulset.yaml)
DRY_RUN_ONLY=(network-policy.yaml hpa.yaml ingress.yaml cdn-config.yaml redis.yaml)

kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

# Cluster-scoped CRDs (validated by the API server on apply).
kubectl apply -f "$K8S_DIR/crd/"

# The API server only *warns* about Pod Security "restricted" violations on
# workload objects (the pods are then rejected later by the workload controller), so
# fail fast on those warnings here. Not --warnings-as-errors: recent API
# servers emit an unrelated, spurious sessionAffinity warning for headless
# Services.
fail_on_pss_warnings() {
  local log="$1"
  cat "$log"
  if grep -q "would violate PodSecurity" "$log"; then
    echo "error: manifests violate the namespace's Pod Security Standard" >&2
    return 1
  fi
}

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

for f in "${CORE[@]}"; do
  sed "s#ghcr.io/saasy-solutions/mockforge:latest#${IMAGE}#" "$K8S_DIR/$f"
  echo "---"
done | kubectl apply -n "$NAMESPACE" -f - >"$log" 2>&1 || { cat "$log"; exit 1; }
fail_on_pss_warnings "$log"

for f in "${DRY_RUN_ONLY[@]}"; do
  kubectl apply --dry-run=server -n "$NAMESPACE" -f "$K8S_DIR/$f" >"$log" 2>&1 || { cat "$log"; exit 1; }
  fail_on_pss_warnings "$log"
done
