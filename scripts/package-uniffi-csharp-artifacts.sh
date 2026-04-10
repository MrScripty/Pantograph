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
    library_name="libpantograph_headless.dylib"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    platform="${PANTOGRAPH_PACKAGE_PLATFORM:-win-x64}"
    library_name="pantograph_headless.dll"
    ;;
  *)
    platform="${PANTOGRAPH_PACKAGE_PLATFORM:-linux-x64}"
    library_name="libpantograph_headless.so"
    ;;
esac

library_path="$cargo_profile_dir/$library_name"
if [[ ! -f "$library_path" ]]; then
  echo "Expected Pantograph headless native library at '$library_path'" >&2
  exit 1
fi

package_root="target/bindings-package"
artifact_dir="$package_root/artifacts"
generated_dir="$package_root/generated/csharp"
csharp_package="$package_root/pantograph-csharp-bindings"
native_package="$package_root/pantograph-headless-native-$platform"

rm -rf "$package_root"
mkdir -p "$artifact_dir" "$generated_dir"

uniffi-bindgen-cs \
  --library \
  --crate pantograph_headless \
  --out-dir "$generated_dir" \
  "$library_path"

generated_binding="$generated_dir/pantograph_headless.cs"
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

write_csharp_manifest() {
  local destination="$1"
  cat > "$destination/manifest.json" <<EOF
{
  "package": "pantograph-csharp-bindings",
  "native_module": "pantograph_headless",
  "required_native_library": "$library_name",
  "platform": "$platform",
  "cargo_profile": "$profile",
  "generated_csharp": "bindings/csharp/pantograph_headless.cs",
  "docs": "docs/headless-native-bindings.md",
  "example": "examples/csharp/Pantograph.DirectRuntimeQuickstart",
  "native_package": "pantograph-headless-native-$platform.zip"
}
EOF
}

write_native_manifest() {
  local destination="$1"
  cat > "$destination/manifest.json" <<EOF
{
  "package": "pantograph-headless-native",
  "native_module": "pantograph_headless",
  "native_library": "$library_name",
  "platform": "$platform",
  "cargo_profile": "$profile",
  "docs": "docs/headless-native-bindings.md",
  "example": "examples/csharp/Pantograph.DirectRuntimeQuickstart"
}
EOF
}

mkdir -p "$csharp_package/bindings/csharp"
install_docs_and_examples "$csharp_package"
cp "$generated_binding" "$csharp_package/bindings/csharp/pantograph_headless.cs"
cp bindings/csharp/PACKAGE-README.md "$csharp_package/README.md"
write_csharp_manifest "$csharp_package"

mkdir -p "$native_package/native/$platform"
install_docs_and_examples "$native_package"
cp "$library_path" "$native_package/native/$platform/$library_name"
cp docs/headless-native-bindings.md "$native_package/README.md"
write_native_manifest "$native_package"

(
  cd "$package_root"
  zip -qr "artifacts/pantograph-csharp-bindings.zip" "pantograph-csharp-bindings"
  zip -qr "artifacts/pantograph-headless-native-$platform.zip" "pantograph-headless-native-$platform"
)

(
  cd "$artifact_dir"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum \
      "pantograph-csharp-bindings.zip" \
      "pantograph-headless-native-$platform.zip" \
      > checksums-sha256.txt
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 \
      "pantograph-csharp-bindings.zip" \
      "pantograph-headless-native-$platform.zip" \
      > checksums-sha256.txt
  else
    echo "Missing required checksum tool: sha256sum or shasum" >&2
    exit 1
  fi
)

echo "Packaged C# bindings: $artifact_dir/pantograph-csharp-bindings.zip"
echo "Packaged native Pantograph library: $artifact_dir/pantograph-headless-native-$platform.zip"
echo "Packaged checksums: $artifact_dir/checksums-sha256.txt"
