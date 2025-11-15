#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${PROJECT_ROOT}"

usage() {
    cat <<'EOF'
Usage: scripts/coverage.sh [--mode MODE] [-- <extra cargo llvm-cov flags...>]

MODE:
  all          Run unit + integration tests (default). Output: target/coverage/
  unit         Run unit tests only.          Output: target/coverage-unit/
  integration  Run integration tests only.   Output: target/coverage-integration/

Any arguments after `--` are passed directly to cargo-llvm-cov.
EOF
}

MODE="all"
EXTRA_ARGS=()

while (($# > 0)); do
    case "$1" in
        -m|--mode)
            if (($# < 2)); then
                echo "--mode requires a value" >&2
                usage
                exit 1
            fi
            MODE="$2"
            shift 2
            ;;
        --mode=*)
            MODE="${1#*=}"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            EXTRA_ARGS+=("$@")
            break
            ;;
        *)
            EXTRA_ARGS+=("$1")
            shift
            ;;
    esac
done

MODE="${MODE,,}"

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

coverage_dir() {
    local suffix="$1"
    if [ -n "${suffix}" ]; then
        echo "${PROJECT_ROOT}/target/coverage-${suffix}"
    else
        echo "${PROJECT_ROOT}/target/coverage"
    fi
}

run_cargo_llvm_cov() {
    local cover_dir="$1"
    shift

    cargo llvm-cov clean --workspace
    cargo llvm-cov --all-features --html --output-dir "${cover_dir}" "$@" "${EXTRA_ARGS[@]}"

    local report="${cover_dir}/html/index.html"
    if [ -f "${report}" ]; then
        echo "coverage report (${MODE}): ${report}"
    else
        echo "coverage report (${MODE}) finished (HTML output directory: ${cover_dir}/html)"
    fi
}

case "${MODE}" in
    all|both|full)
        COVERAGE_DIR="$(coverage_dir "")"
        echo "==> Generating coverage report (unit + integration) into ${COVERAGE_DIR}"
        run_cargo_llvm_cov "${COVERAGE_DIR}" --workspace
        ;;
    unit|units)
        COVERAGE_DIR="$(coverage_dir "unit")"
        echo "==> Generating coverage report (unit tests only) into ${COVERAGE_DIR}"
        run_cargo_llvm_cov "${COVERAGE_DIR}" --lib --bins
        ;;
    integration|integ|cli|e2e)
        TEST_DIR="${PROJECT_ROOT}/tests"
        if [ ! -d "${TEST_DIR}" ]; then
            echo "tests directory (${TEST_DIR}) not found; cannot run integration-only coverage." >&2
            exit 1
        fi
        mapfile -t INTEGRATION_TESTS < <(find "${TEST_DIR}" -maxdepth 1 -type f -name '*.rs' -print | sort)
        if [ "${#INTEGRATION_TESTS[@]}" -eq 0 ]; then
            echo "No integration tests (*.rs) found under ${TEST_DIR}" >&2
            exit 1
        fi

        TEST_ARGS=()
        for test_path in "${INTEGRATION_TESTS[@]}"; do
            test_name="$(basename "${test_path}")"
            test_name="${test_name%.rs}"
            TEST_ARGS+=(--test "${test_name}")
        done

        COVERAGE_DIR="$(coverage_dir "integration")"
        echo "==> Generating coverage report (integration tests only) into ${COVERAGE_DIR}"
        run_cargo_llvm_cov "${COVERAGE_DIR}" --workspace "${TEST_ARGS[@]}"
        ;;
    *)
        echo "Unknown mode: ${MODE}" >&2
        usage
        exit 1
        ;;
esac
