#!/bin/bash
#
# Generate the API documentation for every language binding (C, C++, Python,
# Rust) into docs/<language>/.
#
# This is a thin wrapper around the top-level CMake `docs` target: it configures
# a dedicated build tree, runs the target, and then reports what was actually
# produced. The CMake side is deliberately best-effort (a missing generator
# skips that language with a note); this script is stricter and fails if any
# binding is missing, since generating all of them is the whole point here.
#
# Usage: ./build_docs.sh [--clean] [--keep-going] [--build-dir DIR]
#
# Required tools: cmake, cargo (rustdoc), doxygen (C/C++), sphinx-build (Python).

set -u -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_DIR="${SCRIPT_DIR}/.."
BUILD_DIR="${SRC_DIR}/build-docs"
DOCS_DIR="${SRC_DIR}/docs"
LANGUAGES=(c cpp python rust)

clean=0
keep_going=0

usage() {
    sed -n '2,/^$/s/^# \?//p' "${BASH_SOURCE[0]}"
    exit "${1:-0}"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --clean)      clean=1 ;;
        --keep-going) keep_going=1 ;;
        --build-dir)  shift; [ $# -gt 0 ] || { echo "--build-dir needs an argument" >&2; exit 2; }
                      BUILD_DIR="$1" ;;
        -h|--help)    usage 0 ;;
        *)            echo "Unknown option: $1" >&2; usage 2 ;;
    esac
    shift
done

# --- Tool check ------------------------------------------------------------
# cmake and cargo are hard requirements: without them nothing can be generated
# (the CMake configure step itself looks for cargo with REQUIRED). doxygen and
# sphinx-build only affect individual languages, so they are reported here and
# enforced later via the per-language output check.
missing_required=()
for tool in cmake cargo; do
    command -v "$tool" >/dev/null 2>&1 || missing_required+=("$tool")
done
if [ ${#missing_required[@]} -gt 0 ]; then
    echo "error: missing required tool(s): ${missing_required[*]}" >&2
    exit 1
fi

for tool in doxygen sphinx-build; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "warning: $tool not found - the languages it generates will be skipped" >&2
    fi
done

# --- Generate --------------------------------------------------------------
if [ "$clean" -eq 1 ]; then
    echo "Cleaning ${BUILD_DIR} and ${DOCS_DIR}"
    rm -rf "$BUILD_DIR" "$DOCS_DIR"
fi

# ENPOSE_BUILD_EXAMPLES=OFF: the docs target needs no compiled artifacts, so
# skip configuring (and building) the C/C++ examples.
cmake -S "$SRC_DIR" -B "$BUILD_DIR" -DENPOSE_BUILD_EXAMPLES=OFF >/dev/null || {
    echo "error: cmake configure failed; re-run without output suppression:" >&2
    echo "       cmake -S '$SRC_DIR' -B '$BUILD_DIR' -DENPOSE_BUILD_EXAMPLES=OFF" >&2
    exit 1
}

cmake --build "$BUILD_DIR" --target docs || exit 1

# --- Report ----------------------------------------------------------------
# The CMake script never fails on a skipped language, so check the outputs.
echo
echo "Generated documentation:"
failed=()
for lang in "${LANGUAGES[@]}"; do
    index="${DOCS_DIR}/${lang}/index.html"
    if [ -s "$index" ]; then
        printf '  %-6s %s\n' "$lang" "$index"
    else
        printf '  %-6s MISSING\n' "$lang"
        failed+=("$lang")
        log="${BUILD_DIR}/docs-${lang}.log"
        [ -f "$log" ] && echo "         generator log: $log"
    fi
done

if [ ${#failed[@]} -gt 0 ]; then
    echo
    echo "error: documentation missing for: ${failed[*]}" >&2
    echo "       (c/cpp need doxygen, python needs sphinx-build, rust needs cargo)" >&2
    [ "$keep_going" -eq 1 ] || exit 1
fi
