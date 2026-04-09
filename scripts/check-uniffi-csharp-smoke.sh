#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v uniffi-bindgen-cs >/dev/null 2>&1; then
  echo "Missing required generator: uniffi-bindgen-cs" >&2
  echo "Install a UniFFI 0.28-compatible C# generator, for example uniffi-bindgen-cs 0.9.x." >&2
  exit 1
fi

if ! command -v dotnet >/dev/null 2>&1; then
  echo "Missing required .NET SDK: dotnet" >&2
  exit 1
fi

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

generated_dir="target/uniffi/csharp"
generated_binding="$generated_dir/pantograph_uniffi.cs"
mkdir -p "$generated_dir"

uniffi-bindgen-cs \
  --library \
  --crate pantograph_uniffi \
  --out-dir "$generated_dir" \
  "$library_path"

if [[ ! -f "$generated_binding" ]]; then
  echo "Expected generated C# binding at '$generated_binding'" >&2
  exit 1
fi

require_generated_text() {
  local needle="$1"
  if ! grep -Fq "$needle" "$generated_binding"; then
    echo "Generated C# binding is missing expected text: $needle" >&2
    echo "Generated binding: $generated_binding" >&2
    exit 1
  fi
}

require_generated_text 'public class FfiPantographRuntime'
require_generated_text 'public record FfiEmbeddedRuntimeConfig'
require_generated_text 'Task<String> WorkflowRun(String @requestJson)'
require_generated_text 'Task<String> WorkflowCreateSession(String @requestJson)'

dotnet_root="$(dirname "$(readlink -f "$(command -v dotnet)")")"
sdk_dir="$dotnet_root/sdk"
sdk_version="$(dotnet --version)"
csc_path="$sdk_dir/$sdk_version/Roslyn/bincore/csc.dll"
ref_dir="$(
  find "$dotnet_root/packs/Microsoft.NETCore.App.Ref" \
    -path '*/ref/net*' \
    -type d 2>/dev/null \
  | sort -V \
  | tail -n 1
)"

if [[ ! -f "$csc_path" ]]; then
  echo "Expected Roslyn compiler at '$csc_path'" >&2
  exit 1
fi

if [[ -z "$ref_dir" || ! -d "$ref_dir" ]]; then
  echo "Could not find installed .NET reference assemblies below the dotnet installation." >&2
  exit 1
fi

compile_dir="target/csharp-smoke"
mkdir -p "$compile_dir"

references=()
for reference in "$ref_dir"/*.dll; do
  references+=("-r:$reference")
done

dotnet "$csc_path" \
  -noconfig \
  -unsafe \
  -nullable:enable \
  -langversion:latest \
  -target:exe \
  -out:"$compile_dir/Pantograph.NativeSmoke.dll" \
  "${references[@]}" \
  "$generated_binding" \
  bindings/csharp/Pantograph.NativeSmoke/Program.cs

echo "Verified generated C# UniFFI compile smoke: $generated_binding"
