#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v dotnet >/dev/null 2>&1; then
  echo "Missing required .NET SDK: dotnet" >&2
  exit 1
fi

package_dir="${PANTOGRAPH_CSHARP_PACKAGE_DIR:-target/bindings-package/pantograph-csharp-bindings}"
generated_binding="$package_dir/bindings/csharp/pantograph_uniffi.cs"
quickstart_source="$package_dir/examples/csharp/Pantograph.DirectRuntimeQuickstart/Program.cs"

if [[ ! -f "$generated_binding" ]]; then
  echo "Expected packaged generated binding at '$generated_binding'" >&2
  exit 1
fi

if [[ ! -f "$quickstart_source" ]]; then
  echo "Expected packaged quickstart source at '$quickstart_source'" >&2
  exit 1
fi

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

compile_dir="target/csharp-quickstart-check"
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
  -out:"$compile_dir/Pantograph.DirectRuntimeQuickstart.dll" \
  "${references[@]}" \
  "$generated_binding" \
  "$quickstart_source"

runtime_version="$(
  dotnet --list-runtimes \
  | awk '/^Microsoft\.NETCore\.App / {print $2}' \
  | sort -V \
  | tail -n 1
)"

if [[ -z "$runtime_version" ]]; then
  echo "Could not find an installed Microsoft.NETCore.App runtime." >&2
  exit 1
fi

cat > "$compile_dir/Pantograph.DirectRuntimeQuickstart.runtimeconfig.json" <<EOF
{
  "runtimeOptions": {
    "tfm": "net${runtime_version%%.*}.0",
    "framework": {
      "name": "Microsoft.NETCore.App",
      "version": "$runtime_version"
    }
  }
}
EOF

echo "Verified packaged C# quickstart compiles against: $generated_binding"
