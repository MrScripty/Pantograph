#!/usr/bin/env bash
set -uo pipefail
source /workspace/pantograph-tools/activate.sh
cd "${CHAT_CHECK_CWD:-/workspace/Pantograph}"
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
export XDG_CONFIG_HOME=/workspace/pantograph-cache/chat-codeql-logging-successor-2026-10-06/xdg-config
export XDG_CACHE_HOME=/workspace/pantograph-cache/chat-codeql-logging-successor-2026-10-06/xdg-cache
mkdir -p "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
export PYO3_PYTHON=/workspace/Pantograph/.venv/bin/python
chat_python_lib="$(python -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))')"
export LD_LIBRARY_PATH="$chat_python_lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
log_root=/workspace/pantograph-cache/chat-codeql-logging-successor-2026-10-06
label="$1"
shift
printf '%q ' "$@" > "$log_root/$label.command"
printf '\n' >> "$log_root/$label.command"
timeout 600 "$@" > "$log_root/$label.log" 2>&1
status=$?
printf '%s\n' "$status" > "$log_root/$label.exit"
tail -18 "$log_root/$label.log"
printf 'check=%s exit=%s\n' "$label" "$status"
exit "$status"
