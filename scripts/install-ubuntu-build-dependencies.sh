#!/usr/bin/env bash
set -euo pipefail

# Shared prerequisites for the existing Ubuntu desktop-build CI consumers.
# This intentionally installs system packages; run only on the selected runner.
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  libayatana-appindicator3-dev \
  libglib2.0-dev \
  libgtk-3-dev \
  libjavascriptcoregtk-4.1-dev \
  librsvg2-dev \
  libsoup-3.0-dev \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  libxdo-dev \
  pkg-config \
  protobuf-compiler
