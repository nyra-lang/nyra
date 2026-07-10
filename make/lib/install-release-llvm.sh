#!/usr/bin/env sh
# Install libclang for release packaging (CI + local). On Apple Silicon, cross-building
# x86_64-apple-darwin needs x86_64 libclang — Homebrew LLVM is arm64-only.
set -eu

TRIPLE="${1:?usage: install-release-llvm.sh <target-triple>}"

set_env() {
  key="$1"
  val="$2"
  if [ -n "${GITHUB_ENV:-}" ]; then
    printf '%s=%s\n' "$key" "$val" >> "$GITHUB_ENV"
  fi
  # shellcheck disable=SC2163
  export "$key=$val"
}

# rust-cache / prior brew LLVM can leave arm64 link paths in clang-sys build output.
clean_clang_sys_build_cache() {
  triple="$1"
  if [ "$triple" != "x86_64-apple-darwin" ]; then
    return 0
  fi
  if [ "$(uname -s)" != "Darwin" ] || [ "$(uname -m)" != "arm64" ]; then
    return 0
  fi
  echo "install-release-llvm: clearing stale clang-sys cache for ${triple} ..."
  rm -rf "target/${triple}/release/build/clang-sys-"*
  rm -rf "target/${triple}/release/deps/libclang-"*
  rm -rf "target/${triple}/release/deps/libclang_sys-"*
  rm -rf "target/${triple}/release/.fingerprint/clang-sys-"*
  rm -rf "target/${triple}/release/.fingerprint/libclang-"*
}

case "$TRIPLE" in
  x86_64-apple-darwin)
    LLVM_VER=15.0.7
    LLVM_DIR="clang+llvm-${LLVM_VER}-x86_64-apple-darwin21.0"
    LLVM_ROOT="${HOME}/${LLVM_DIR}"
    if [ ! -f "${LLVM_ROOT}/lib/libclang.dylib" ]; then
      echo "install-release-llvm: fetching ${LLVM_DIR} ..."
      curl -fsSL \
        "https://github.com/llvm/llvm-project/releases/download/llvmorg-${LLVM_VER}/${LLVM_DIR}.tar.xz" \
        | tar xJ -C "$HOME"
    fi
    set_env LIBCLANG_PATH "${LLVM_ROOT}/lib"
    set_env LLVM_CONFIG_PATH "${LLVM_ROOT}/bin/llvm-config"
    set_env PATH "${LLVM_ROOT}/bin:${PATH}"
    set_env DYLD_LIBRARY_PATH "${LLVM_ROOT}/lib:${DYLD_LIBRARY_PATH:-}"
    set_env CARGO_TARGET_X86_64_APPLE_DARWIN_RUSTFLAGS "-L native=${LLVM_ROOT}/lib"
    clean_clang_sys_build_cache "$TRIPLE"
    ;;
  aarch64-apple-darwin)
    if [ "$(uname -s)" = "Darwin" ]; then
      if command -v brew >/dev/null 2>&1; then
        brew install llvm 2>/dev/null || true
        llvm_lib="$(brew --prefix llvm)/lib"
        set_env LIBCLANG_PATH "$llvm_lib"
        set_env DYLD_LIBRARY_PATH "${llvm_lib}:${DYLD_LIBRARY_PATH:-}"
      fi
    fi
    ;;
  *)
    # Linux / Windows: system deps from nyra-ci-setup or release.yml apt/llvm-action.
    ;;
esac
