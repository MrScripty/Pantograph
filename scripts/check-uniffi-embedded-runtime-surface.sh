#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p pantograph-uniffi

case "$(uname -s)" in
  Darwin)
    library_path="target/debug/libpantograph_uniffi.dylib"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    library_path="target/debug/pantograph_uniffi.dll"
    ;;
  *)
    library_path="target/debug/libpantograph_uniffi.so"
    ;;
esac

if [[ ! -f "$library_path" ]]; then
  echo "Expected UniFFI library at '$library_path'" >&2
  exit 1
fi

repr_dir="target/uniffi"
repr_path="$repr_dir/pantograph-uniffi.repr.txt"
mkdir -p "$repr_dir"

cargo run -p pantograph-uniffi --bin pantograph-uniffi-bindgen --features cli -- \
  print-repr "$library_path" > "$repr_path"

require_metadata() {
  local needle="$1"
  if ! grep -Fq "$needle" "$repr_path"; then
    echo "UniFFI metadata is missing expected binding item: $needle" >&2
    echo "Metadata dump: $repr_path" >&2
    exit 1
  fi
}

require_metadata 'name: "FfiEmbeddedRuntimeConfig"'
require_metadata 'name: "FfiPantographRuntime"'
require_metadata 'name: "workflow_run"'
require_metadata 'name: "workflow_get_capabilities"'
require_metadata 'name: "workflow_get_io"'
require_metadata 'name: "workflow_preflight"'
require_metadata 'name: "workflow_create_session"'
require_metadata 'name: "workflow_run_session"'
require_metadata 'name: "workflow_close_session"'
require_metadata 'name: "workflow_get_session_status"'
require_metadata 'name: "workflow_list_session_queue"'
require_metadata 'name: "workflow_cancel_session_queue_item"'
require_metadata 'name: "workflow_reprioritize_session_queue_item"'
require_metadata 'name: "workflow_set_session_keep_alive"'
require_metadata 'name: "shutdown"'

echo "Verified embedded runtime surface in UniFFI metadata: $repr_path"
