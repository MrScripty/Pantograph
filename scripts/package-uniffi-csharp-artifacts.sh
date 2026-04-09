#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v uniffi-bindgen-cs >/dev/null 2>&1; then
  echo "Missing required generator: uniffi-bindgen-cs" >&2
  echo "Install a UniFFI 0.28-compatible C# generator, for example uniffi-bindgen-cs 0.9.x." >&2
  exit 1
fi

if ! command -v zip >/dev/null 2>&1; then
  echo "Missing required archiver: zip" >&2
  exit 1
fi

profile="${PANTOGRAPH_PACKAGE_PROFILE:-release}"
if [[ "$profile" == "release" ]]; then
  cargo build -p pantograph-uniffi --release
  cargo_profile_dir="target/release"
else
  cargo build -p pantograph-uniffi
  cargo_profile_dir="target/debug"
fi

case "$(uname -s)" in
  Darwin)
    platform="${PANTOGRAPH_PACKAGE_PLATFORM:-osx}"
    library_name="libpantograph_uniffi.dylib"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    platform="${PANTOGRAPH_PACKAGE_PLATFORM:-win-x64}"
    library_name="pantograph_uniffi.dll"
    ;;
  *)
    platform="${PANTOGRAPH_PACKAGE_PLATFORM:-linux-x64}"
    library_name="libpantograph_uniffi.so"
    ;;
esac

library_path="$cargo_profile_dir/$library_name"
if [[ ! -f "$library_path" ]]; then
  echo "Expected UniFFI native library at '$library_path'" >&2
  exit 1
fi

package_root="target/bindings-package"
artifact_dir="$package_root/artifacts"
generated_dir="$package_root/generated/csharp"
csharp_package="$package_root/pantograph-csharp-bindings"
native_package="$package_root/pantograph-native-runtime-$platform"

rm -rf "$package_root"
mkdir -p "$artifact_dir" "$generated_dir"

uniffi-bindgen-cs \
  --library \
  --crate pantograph_uniffi \
  --out-dir "$generated_dir" \
  "$library_path"

generated_binding="$generated_dir/pantograph_uniffi.cs"
if [[ ! -f "$generated_binding" ]]; then
  echo "Expected generated C# binding at '$generated_binding'" >&2
  exit 1
fi

install_docs_and_examples() {
  local destination="$1"
  mkdir -p \
    "$destination/docs" \
    "$destination/examples/csharp" \
    "$destination/README-assets"

  cp docs/headless-native-bindings.md "$destination/docs/"
  cp -R bindings/csharp/Pantograph.DirectRuntimeQuickstart \
    "$destination/examples/csharp/"
}

write_manifest() {
  local destination="$1"
  local kind="$2"
  cat > "$destination/manifest.json" <<EOF
{
  "package": "$kind",
  "uniffi_crate": "pantograph_uniffi",
  "native_library": "$library_name",
  "platform": "$platform",
  "cargo_profile": "$profile",
  "generated_csharp": "bindings/csharp/pantograph_uniffi.cs",
  "docs": "docs/headless-native-bindings.md",
  "example": "examples/csharp/Pantograph.DirectRuntimeQuickstart"
}
EOF
}

mkdir -p "$csharp_package/bindings/csharp"
install_docs_and_examples "$csharp_package"
cp "$generated_binding" "$csharp_package/bindings/csharp/pantograph_uniffi.cs"
cp bindings/csharp/PACKAGE-README.md "$csharp_package/README.md"
write_manifest "$csharp_package" "pantograph-csharp-bindings"

mkdir -p "$native_package/native/$platform"
install_docs_and_examples "$native_package"
cp "$library_path" "$native_package/native/$platform/$library_name"
cp docs/headless-native-bindings.md "$native_package/README.md"
write_manifest "$native_package" "pantograph-native-runtime"

(
  cd "$package_root"
  zip -qr "artifacts/pantograph-csharp-bindings.zip" "pantograph-csharp-bindings"
  zip -qr "artifacts/pantograph-native-runtime-$platform.zip" "pantograph-native-runtime-$platform"
)

echo "Packaged C# bindings: $artifact_dir/pantograph-csharp-bindings.zip"
echo "Packaged native runtime: $artifact_dir/pantograph-native-runtime-$platform.zip"
