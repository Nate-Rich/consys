#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: bench/run_hyperfine.sh [OPTIONS]

Run standardized consys hyperfine benchmarks. The default condition is warm,
which represents normal repeated CLI use.

Available conditions:
  warm     normal repeated CLI use; recommended release result
  fast     hot-cache / best-case repeated execution
  mid      lightly disturbed cache between measurements

Options:
  --out DIR             Output directory (default: bench-output/hyperfine-YYYYmmdd-HHMMSS)
  --binary PATH         consys binary to benchmark (default: ./target/release/consys)
  --conditions LIST     Comma list: warm,fast,mid (default: warm)
  --groups LIST         Comma list: flags,tools (default: flags,tools)
  --min-runs N          Override hyperfine --min-runs for every condition
  --warmup N            Override hyperfine --warmup for every condition
  --no-build            Do not run cargo build --release first
  --no-checks           Skip correctness checks before timing
  --dry-run             Print planned commands without running hyperfine
  -h, --help            Show this help

Examples:
  bench/run_hyperfine.sh
  bench/run_hyperfine.sh --conditions warm --groups flags
  bench/run_hyperfine.sh --conditions fast,warm,mid

Notes:
  - Raw JSON/Markdown exports and captured stdout go under the output directory.
  - Token counting is intentionally not run here; use bench/count_tokens_fetch.py separately.
  - Missing comparison tools are skipped automatically.
USAGE
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

STAMP="$(date +%Y%m%d-%H%M%S)"
OUT_DIR="bench-output/hyperfine-$STAMP"
BINARY="./target/release/consys"
CONDITIONS="warm"
BENCH_GROUPS="flags,tools"
MIN_RUNS_OVERRIDE=""
WARMUP_OVERRIDE=""
DO_BUILD=1
DO_CHECKS=1
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      OUT_DIR="${2:?missing value for --out}"
      shift 2
      ;;
    --binary)
      BINARY="${2:?missing value for --binary}"
      shift 2
      ;;
    --conditions)
      CONDITIONS="${2:?missing value for --conditions}"
      shift 2
      ;;
    --groups)
      BENCH_GROUPS="${2:?missing value for --groups}"
      shift 2
      ;;
    --min-runs)
      MIN_RUNS_OVERRIDE="${2:?missing value for --min-runs}"
      shift 2
      ;;
    --warmup)
      WARMUP_OVERRIDE="${2:?missing value for --warmup}"
      shift 2
      ;;
    --no-build)
      DO_BUILD=0
      shift
      ;;
    --no-checks)
      DO_CHECKS=0
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

IFS=',' read -r -a CONDITION_LIST <<< "$CONDITIONS"
IFS=',' read -r -a GROUP_LIST <<< "$BENCH_GROUPS"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

has_item() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    [[ "$item" == "$needle" ]] && return 0
  done
  return 1
}

condition_settings() {
  local condition="$1"
  case "$condition" in
    fast)
      WARMUP="50"
      MIN_RUNS="2000"
      PREPARE=""
      ;;
    warm)
      WARMUP="10"
      MIN_RUNS="1000"
      PREPARE=""
      ;;
    mid)
      WARMUP="3"
      MIN_RUNS="300"
      PREPARE="sh -c 'sync; sleep 0.05'"
      ;;
    *)
      echo "error: unknown condition: $condition" >&2
      exit 2
      ;;
  esac

  if [[ -n "$WARMUP_OVERRIDE" ]]; then
    WARMUP="$WARMUP_OVERRIDE"
  fi
  if [[ -n "$MIN_RUNS_OVERRIDE" ]]; then
    MIN_RUNS="$MIN_RUNS_OVERRIDE"
  fi
}

log_argv() {
  local arg first=1
  printf "+"
  for arg in "$@"; do
    if [[ "$first" -eq 1 ]]; then
      first=0
    else
      printf " "
    fi
    # Print every argv item as a single-quoted shell token so dry-run output can
    # be copied directly and benchmarked commands are visibly passed to
    # hyperfine as single command strings, e.g. './target/release/consys -t'.
    printf "'%s'" "${arg//\'/\'\\\'\'}"
  done
  printf "\n"
}

run_cmd() {
  log_argv "$@"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    "$@"
  fi
}

run_shell() {
  echo "+ $*"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    bash -c "$*"
  fi
}

capture_shell() {
  local file="$1"
  shift
  local command="$*"
  echo "+ $command > $file"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    bash -c "$command" > "$file" 2>&1 || true
  fi
}

hyperfine_run() {
  local condition="$1"
  local group="$2"
  shift 2
  local commands=("$@")
  local stem="$OUT_DIR/${condition}-${group}"
  local args=(-N --warmup "$WARMUP" --min-runs "$MIN_RUNS")

  if [[ -n "$PREPARE" ]]; then
    args+=(--prepare "$PREPARE")
  fi

  args+=(--export-json "$stem.json" --export-markdown "$stem.md")
  args+=("${commands[@]}")

  echo
  echo "== $condition / $group =="
  echo "warmup=$WARMUP min_runs=$MIN_RUNS prepare=${PREPARE:-none}"
  run_cmd hyperfine "${args[@]}"
}

validate_config() {
  local condition group
  for condition in "${CONDITION_LIST[@]}"; do
    case "$condition" in fast|warm|mid) ;; *) echo "error: invalid condition: $condition" >&2; exit 2 ;; esac
  done
  for group in "${GROUP_LIST[@]}"; do
    case "$group" in flags|tools) ;; *) echo "error: invalid group: $group" >&2; exit 2 ;; esac
  done
}

comparison_commands() {
  local commands=("$BINARY" "$BINARY -dgput")
  local tool
  for tool in microfetch nitch fastfetch pfetch macchina neofetch; do
    if command -v "$tool" >/dev/null 2>&1; then
      commands+=("$tool")
    else
      echo "skip: comparison tool not installed: $tool" >&2
    fi
  done
  printf '%s\0' "${commands[@]}"
}

capture_environment() {
  local env_file="$OUT_DIR/environment.txt"
  echo "+ capture environment > $env_file"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    return
  fi
  {
    echo "# consys benchmark environment"
    echo "timestamp: $(date -Is)"
    echo "binary: $BINARY"
    echo "conditions: $CONDITIONS"
    echo "groups: $BENCH_GROUPS"
    echo
    echo "## versions"
    uname -srmo || true
    rustc --version || true
    cargo --version || true
    hyperfine --version || true
    python3 --version || true
    "$BINARY" --version || true
    echo
    echo "## consys all-fields sample"
    "$BINARY" -dgput || true
    echo
    echo "## filesystem"
    findmnt -no FSTYPE,OPTIONS / || true
    echo
    echo "## cpu"
    lscpu 2>/dev/null | sed -n '1,30p' || true
    echo
    echo "## governor / turbo"
    cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || true
    cat /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || true
  } > "$env_file"
}

capture_outputs() {
  echo "+ capture stdout outputs under $OUT_DIR"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    return
  fi

  mkdir -p "$OUT_DIR/stdout"
  capture_shell "$OUT_DIR/stdout/consys.txt" "$BINARY"
  capture_shell "$OUT_DIR/stdout/consys-all.txt" "$BINARY -dgput"

  local tool
  for tool in microfetch nitch fastfetch pfetch macchina neofetch; do
    if command -v "$tool" >/dev/null 2>&1; then
      capture_shell "$OUT_DIR/stdout/${tool}.txt" "$tool"
    fi
  done
}

main() {
  validate_config
  require_cmd cargo
  require_cmd hyperfine

  if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$OUT_DIR"
  else
    echo "+ mkdir -p $OUT_DIR"
  fi

  if [[ "$DO_BUILD" -eq 1 ]]; then
    run_cmd cargo build --release
  fi

  if [[ ! -x "$BINARY" && "$DRY_RUN" -eq 0 ]]; then
    echo "error: binary not found or not executable: $BINARY" >&2
    exit 1
  fi

  if [[ "$DO_CHECKS" -eq 1 ]]; then
    run_cmd cargo fmt -- --check
    run_cmd cargo clippy --all-targets --all-features -- -D warnings
    run_cmd cargo test --quiet
    run_cmd "$BINARY" --version
    run_cmd "$BINARY" -dgput
    run_shell "$BINARY -x >/dev/null 2>&1; test \$? -eq 2"
  fi

  capture_environment
  capture_outputs

  local condition group
  for condition in "${CONDITION_LIST[@]}"; do
    condition_settings "$condition"
    for group in "${GROUP_LIST[@]}"; do
      case "$group" in
        flags)
          hyperfine_run "$condition" "$group" \
            "$BINARY" \
            "$BINARY -t" \
            "$BINARY -u" \
            "$BINARY -d" \
            "$BINARY -g" \
            "$BINARY -p" \
            "$BINARY -dg" \
            "$BINARY -pu" \
            "$BINARY -dgput"
          ;;
        tools)
          local commands=()
          while IFS= read -r -d '' cmd; do
            commands+=("$cmd")
          done < <(comparison_commands)
          hyperfine_run "$condition" "$group" "${commands[@]}"
          ;;
      esac
    done
  done

  if [[ "$DRY_RUN" -eq 0 ]]; then
    cat > "$OUT_DIR/README.txt" <<EOF
consys benchmark run
====================

Generated by: bench/run_hyperfine.sh
Timestamp: $STAMP
Conditions: $CONDITIONS
Groups: $BENCH_GROUPS

Important files:
- environment.txt: machine/tool/context capture
- stdout/*.txt: exact output used for token counting
- *-flags.json / *-flags.md: consys per-flag hyperfine results
- *-tools.json / *-tools.md: comparison-tool hyperfine results

Token counts are intentionally separate from the benchmark run:
python3 bench/count_tokens_fetch.py "$OUT_DIR"/stdout/*.txt > "$OUT_DIR"/tokens.txt
EOF
  else
    echo "+ write run README > $OUT_DIR/README.txt"
  fi

  echo
  echo "Done. Benchmark artifacts written to: $OUT_DIR"
}

main "$@"
