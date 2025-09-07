#!/usr/bin/env bash
set -euo pipefail

# Rebind the Agent Docs workdir PVC to the node chosen by pod scheduling.
# This helps when a local-path PV was created on the wrong node and pods are pinned elsewhere.
#
# Usage: scripts/rebind-workdir-pvc.sh [-n <namespace>]
# Default namespace: agent-docs
#
# Notes for Argo CD:
# - If Auto-Sync is enabled, consider suspending the app during this operation:
#   argocd app suspend <app-name>
# - Or run quickly; Argo may fight the scale to 0 while we delete the PVC.

NS="agent-docs"
while [[ $# -gt 0 ]]; do
  case "$1" in
    -n|--namespace) NS="$2"; shift 2;;
    -h|--help) echo "Usage: $0 [-n <namespace>]"; exit 0;;
    *) echo "Unknown arg: $1"; exit 1;;
  esac
done

echo "[INFO] Namespace: $NS"

server_dep=$(kubectl -n "$NS" get deploy -l app.kubernetes.io/name=agent-docs-server -o name | grep -v worker | head -n1 || true)
worker_dep=$(kubectl -n "$NS" get deploy -l app.kubernetes.io/component=worker -o name | head -n1 || true)

if [[ -z "$server_dep" ]]; then
  echo "[ERROR] Could not find server Deployment with label app.kubernetes.io/name=agent-docs-server" >&2
  exit 1
fi
if [[ -z "$worker_dep" ]]; then
  echo "[ERROR] Could not find worker Deployment with label app.kubernetes.io/component=worker" >&2
  exit 1
fi

pvc_name=$(kubectl -n "$NS" get pvc -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}' | grep -E "workdir$" | head -n1 || true)
if [[ -z "$pvc_name" ]]; then
  echo "[ERROR] Could not find a PVC ending with '-workdir' in namespace $NS" >&2
  kubectl -n "$NS" get pvc || true
  exit 1
fi

echo "[INFO] Server deployment: $server_dep"
echo "[INFO] Worker deployment: $worker_dep"
echo "[INFO] Workdir PVC: $pvc_name"

echo "[STEP] Scaling deployments to 0 replicas"
kubectl -n "$NS" scale "$server_dep" --replicas=0 || true
kubectl -n "$NS" scale "$worker_dep" --replicas=0 || true

echo "[STEP] Waiting for pods to terminate"
kubectl -n "$NS" wait --for=delete pod -l app.kubernetes.io/name=agent-docs-server --timeout=120s || true

echo "[STEP] Deleting PVC $pvc_name (this will delete the bound PV for local-path)"
kubectl -n "$NS" delete pvc "$pvc_name"

echo "[STEP] Scaling deployments back to 1 replica"
kubectl -n "$NS" scale "$server_dep" --replicas=1
kubectl -n "$NS" scale "$worker_dep" --replicas=1

echo "[INFO] Done. The new PV should be provisioned on the node where the pods schedule (requires StorageClass volumeBindingMode=WaitForFirstConsumer)."
echo "[NEXT] Verify with: kubectl -n $NS get pvc && kubectl get pv | grep $(kubectl -n $NS get pvc $pvc_name -o jsonpath='{.spec.volumeName}' 2>/dev/null || echo '<new-pv>')"

