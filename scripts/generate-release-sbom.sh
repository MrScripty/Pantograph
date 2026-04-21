#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v syft >/dev/null 2>&1; then
  echo "Missing required release tool: syft" >&2
  echo "Install syft before generating release SBOMs." >&2
  exit 1
fi

version="${1:-}"
if [[ -z "$version" ]]; then
  version="$(node -e "process.stdout.write(JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version)")"
fi

version="${version#v}"
if [[ -z "$version" ]]; then
  echo "Unable to determine release version." >&2
  exit 1
fi

output_dir="$ROOT_DIR/target/release-artifacts"
output_file="$output_dir/pantograph-${version}-sbom.cdx.json"

mkdir -p "$output_dir"
syft dir:"$ROOT_DIR" -o cyclonedx-json="$output_file"
echo "$output_file"
