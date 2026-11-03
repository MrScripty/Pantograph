#!/usr/bin/env bash
set -euo pipefail

# The repository selects its map here; the caller must select the change input.
exec node "$(dirname "${BASH_SOURCE[0]}")/check-decision-traceability.mjs" \
  --map scripts/decision-traceability-map.json
