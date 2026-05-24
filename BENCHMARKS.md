# Benchmarks

consys is meant to be small, predictable, and cheap to run. The benchmark suite measures that contract without adding decoration or benchmark-only behavior to the program.

The public benchmark target is normal repeated CLI use: a built release binary run from the shell with warm filesystem and package-manager state.

---

## v1.1.0 release run

Source artifacts:

```text
bench-output/hyperfine-20260524-094350/
```

Profile:

```text
warm: hyperfine -N --warmup 10 --min-runs 1000
```

Token counts were run after timing, not inside the `hyperfine` benchmark.

### Environment

| Field | Value |
|:--|:--|
| Machine class | Intel laptop, 2 cores / 4 threads |
| OS | Linux Mint 22.3 |
| Kernel | Linux 6.17 |
| CPU | Intel Core i7-7567U class |
| Filesystem | ext4 |
| GPU | integrated Intel graphics |
| Rust | rustc 1.95.0, cargo 1.95.0 |
| hyperfine | 1.18.0 |
| Python | 3.12.3 |
| CPU governor / turbo | powersave / turbo enabled |
| consys | 1.1.0 |
| Run date | 2026-05-24 |

### Speed — consys per-flag cost

| Command | Mean | Stddev | Min | Max |
|:--|--:|--:|--:|--:|
| `./target/release/consys` | 0.803 ms | 0.204 ms | 0.668 ms | 2.243 ms |
| `./target/release/consys -t` | 0.823 ms | 0.199 ms | 0.688 ms | 2.529 ms |
| `./target/release/consys -u` | 0.810 ms | 0.225 ms | 0.676 ms | 2.251 ms |
| `./target/release/consys -d` | 0.804 ms | 0.218 ms | 0.670 ms | 2.537 ms |
| `./target/release/consys -g` | 4.188 ms | 0.404 ms | 3.806 ms | 6.867 ms |
| `./target/release/consys -p` | 4.306 ms | 0.600 ms | 3.864 ms | 7.586 ms |
| `./target/release/consys -dg` | 4.237 ms | 0.572 ms | 3.812 ms | 7.734 ms |
| `./target/release/consys -pu` | 4.298 ms | 0.600 ms | 3.858 ms | 6.857 ms |
| `./target/release/consys -dgput` | 7.663 ms | 0.730 ms | 7.036 ms | 10.895 ms |

Base output, local time, uptime, and disk usage are all effectively sub-millisecond in this run. GPU lookup and package counting are the cost centers. All-fields output remains under 8 ms warm-cache.

### Speed — tool comparison

| Command | Mean | Stddev | Min | Max |
|:--|--:|--:|--:|--:|
| `microfetch` | 0.683 ms | 0.196 ms | 0.558 ms | 1.941 ms |
| `nitch` | 0.808 ms | 0.217 ms | 0.668 ms | 2.093 ms |
| `./target/release/consys` | 0.817 ms | 0.199 ms | 0.670 ms | 2.173 ms |
| `./target/release/consys -dgput` | 7.666 ms | 0.686 ms | 6.987 ms | 11.059 ms |
| `fastfetch` | 8.919 ms | 7.289 ms | 6.738 ms | 91.945 ms |
| `pfetch` | 38.525 ms | 2.310 ms | 34.436 ms | 46.349 ms |
| `macchina` | 47.782 ms | 17.048 ms | 38.733 ms | 156.143 ms |
| `neofetch` | 252.355 ms | 2.527 ms | 245.990 ms | 266.538 ms |

microfetch is faster than base consys in this run. nitch and base consys are effectively tied. Base consys is much faster than fastfetch, pfetch, macchina, and neofetch. All-fields consys is still faster than fastfetch, pfetch, macchina, and neofetch.

### Output size and tokens

Tokenizer: `cl100k_base`

| Output | Tokens | Chars | Lines |
|:--|--:|--:|--:|
| `consys` | 79 | 179 | 6 |
| `consys-all` | 146 | 327 | 11 |
| `pfetch` | 351 | 581 | 15 |
| `nitch` | 418 | 651 | 19 |
| `microfetch` | 649 | 907 | 11 |
| `fastfetch` | 914 | 1936 | 48 |
| `macchina` | 1346 | 2789 | 20 |
| `neofetch` | 1443 | 2534 | 42 |

All-fields consys uses less than half the tokens of every comparison output captured here.

---

## Requirements

```sh
cargo --version
rustc --version
hyperfine --version
python3 --version
```

Optional comparison tools are included only when installed:

```sh
microfetch --version || true
nitch --version || true
fastfetch --version || true
pfetch --version || true
macchina --version || true
neofetch --version || true
```

---

## Build and correctness checks

Build the release binary first:

```sh
cargo build --release
./target/release/consys --version
./target/release/consys -dgput
```

Before timing, run the same checks used by the benchmark script:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --quiet
./target/release/consys --version
./target/release/consys -dgput
./target/release/consys -x; test $? -eq 2
```

Do not benchmark a binary that fails these checks.

---

## Run the benchmark suite

Use the benchmark runner instead of hand-writing each `hyperfine` command:

```sh
bench/run_hyperfine.sh
```

The default run builds the release binary, runs correctness checks, captures the output used for token counting, skips comparison tools that are not installed, and writes artifacts under:

```text
bench-output/hyperfine-YYYYmmdd-HHMMSS/
```

Useful options:

```sh
# Warm-cache benchmark suite; this is also the default
bench/run_hyperfine.sh --conditions warm

# Only consys per-flag measurements
bench/run_hyperfine.sh --groups flags

# Only comparison-tool measurements
bench/run_hyperfine.sh --groups tools

# Preview the commands without running hyperfine
bench/run_hyperfine.sh --dry-run
```

Release notes should use the warm results unless another condition is clearly labeled and justified.

---

## Benchmark groups

The `flags` group measures consys per-flag cost:

```sh
./target/release/consys
./target/release/consys -t
./target/release/consys -u
./target/release/consys -d
./target/release/consys -g
./target/release/consys -p
./target/release/consys -dg
./target/release/consys -pu
./target/release/consys -dgput
```

The `tools` group compares consys to installed fetch tools:

```sh
./target/release/consys
./target/release/consys -dgput
microfetch
nitch
fastfetch
pfetch
macchina
neofetch
```

Missing comparison tools are skipped automatically.

---

## Output-size and token checks

Speed is not the only useful measurement for a system-info tool. consys is also intended to produce compact output for logs, scripts, text-to-speech, and language-model context.

The benchmark runner captures stdout for each command, but token counting is run afterward so tokenizer work does not affect timing:

```sh
python3 bench/count_tokens_fetch.py bench-output/hyperfine-*/stdout/*.txt
```

Use the same tokenizer for every compared output. The script uses `tiktoken` with `cl100k_base` when available and falls back to a simple regex tokenizer otherwise.

---

## Reading the results

For release notes, the main files are:

```text
bench-output/hyperfine-*/warm-flags.md
bench-output/hyperfine-*/warm-tools.md
bench-output/hyperfine-*/tokens.txt
```

A useful public summary should be small:

- consys default output timing
- consys all-fields timing
- comparison-tool timing when those tools are installed
- output character, line, and token counts

Keep machine-specific raw logs out of the project README. The README should stay focused on what consys does, how to install it, and how to verify a release binary.
