#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${PROJECT_ROOT}"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "cargo-llvm-cov not found, installing it..." >&2
    cargo install cargo-llvm-cov --locked
fi

if ! command -v rustup >/dev/null 2>&1; then
    echo "rustup is required to ensure llvm-tools-preview is installed." >&2
    exit 1
fi

if ! rustup component list --installed | grep -Eq '^llvm-tools(-preview)?'; then
    echo "Adding llvm-tools-preview component..." >&2
    rustup component add llvm-tools-preview
fi

echo "==> Generating coverage report into target/coverage/"
COVERAGE_DIR="${PROJECT_ROOT}/target/coverage"
cargo llvm-cov --workspace --all-features --html --output-dir "${COVERAGE_DIR}" "$@"
REPORT="${COVERAGE_DIR}/html/index.html"
if [ -f "${REPORT}" ]; then
    echo "coverage report: ${REPORT}"
else
    echo "coverage report finished (HTML output directory: ${COVERAGE_DIR}/html)"
fi
