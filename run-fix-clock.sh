#!/usr/bin/env bash
set -euo pipefail
cd "${BASH_SOURCE%/*}/fix-clock"

ulimit -n 1024

echo "__VMM_START__"
exec firecracker --no-api --config-file ../vm_config.json --level "${FC_LOG_LEVEL:-Error}"
