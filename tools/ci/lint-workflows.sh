#!/usr/bin/env bash
set -euo pipefail

install_dir="$RUNNER_TEMP/actionlint-1.7.12"
mkdir -p "$install_dir"
archive="$install_dir/actionlint.tar.gz"
curl --fail --location --retry 3 \
  https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz \
  --output "$archive"
echo "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8  $archive" | sha256sum --check
tar -xzf "$archive" -C "$install_dir" actionlint
"$install_dir/actionlint" -color
