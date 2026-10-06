#!/usr/bin/env bash
set -euo pipefail
cd "${BASH_SOURCE%/*}/../${1:?usage: $0 <patch|fix-clock>}"

API_SOCKET="${API_SOCKET:-${XDG_RUNTIME_DIR:-/tmp}/firecracker-helloworld.sock}"
FC_API=../annexe/fc-api/target/x86_64-unknown-linux-gnu/release/fc-api

ulimit -n 1024
rm -f "$API_SOCKET"

echo "__VMM_START__"
firecracker --api-sock "$API_SOCKET" --level "${FC_LOG_LEVEL:-Error}" &
fc_pid=$!
trap 'kill "$fc_pid" 2>/dev/null || true; rm -f "$API_SOCKET"' EXIT

"$FC_API" "$API_SOCKET" ../vm_config.json
wait "$fc_pid"
