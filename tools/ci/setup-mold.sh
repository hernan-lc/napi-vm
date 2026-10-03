#!/usr/bin/env bash
set -euo pipefail

case "${RUNNER_ARCH}" in
  X64)
    mold_arch=x86_64
    mold_sha=4c999e19ffa31afa5aa429c679b665d5e2ca5a6b6832ad4b79668e8dcf3d8ec1
    ;;
  ARM64)
    mold_arch=aarch64
    mold_sha=c799b9ccae8728793da2186718fbe53b76400a9da396184fac0c64aa3298ec37
    ;;
  *) echo "Unsupported Linux runner architecture: ${RUNNER_ARCH}" >&2; exit 1 ;;
esac
mold_dir="$HOME/.cache/napi-vm-ci/mold-2.40.4"
if [[ ! -x "$mold_dir/bin/mold" ]]; then
  archive="$RUNNER_TEMP/mold.tar.gz"
  curl --fail --location --retry 3 \
    "https://github.com/rui314/mold/releases/download/v2.40.4/mold-2.40.4-${mold_arch}-linux.tar.gz" \
    --output "$archive"
  echo "$mold_sha  $archive" | sha256sum --check
  mkdir -p "$mold_dir"
  tar -xzf "$archive" --strip-components=1 -C "$mold_dir"
fi
echo "$mold_dir/bin" >> "$GITHUB_PATH"
# Target-specific flags leave wasm-ld and musl's self-contained linker intact.
for target in X86_64_UNKNOWN_LINUX_GNU AARCH64_UNKNOWN_LINUX_GNU; do
  flag_var="CARGO_TARGET_${target}_RUSTFLAGS"
  printf '%s=%s -C link-arg=-fuse-ld=mold\n' "$flag_var" "${!flag_var:-}" >> "$GITHUB_ENV"
done
"$mold_dir/bin/mold" --version
