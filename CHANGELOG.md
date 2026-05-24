# Changelog

All notable changes to consys are documented here.

---

## v1.1.0 — 2026-05-24

v1.1.0 tightens consys around its core contract: small, stable, parseable system context with less subprocess overhead and clearer behavior for scripts.

### Changed

- `-t` now prints current local time instead of elapsed execution time.

  v1.0.0:

  ```text
  time:     47.694664ms
  ```

  v1.1.0:

  ```text
  time:     2026-01-15 09:30:00 UTC-05:00
  ```

- `-d` now reads disk usage through `statvfs64` instead of spawning `df`.
- `-g` now reads GPU information from sysfs and `pci.ids` instead of spawning `lspci`.
- `-p` counts installed packages more carefully:
  - dpkg counts only package blocks with `install ok installed` status.
  - snap counts current applications rather than retained revision blobs.
  - flatpak counts both system and user app directories.
- Short flags can now be clustered. `consys -dgput` is equivalent to `consys -d -g -p -u -t`.
- Unknown short flags now fail loudly with exit code 2 instead of silently producing base output.
- Output is written through one buffered stdout writer instead of separate `println!` calls.
- Release builds now use a tuned release profile: LTO, one codegen unit, stripped symbols, and `panic = "abort"`.

### Added

- `libc` dependency for correct Linux FFI bindings.
- Multi-GPU output uses indexed keys (`gpu0:`, `gpu1:`, ...), avoiding duplicate `gpu:` keys.
- Benchmark runner and documentation for the v1.1.0 release battery.
- v1.1.0 warm-cache benchmark results:
  - base `consys`: about 0.8 ms and 79 `cl100k_base` tokens on the benchmark machine.
  - all-fields `consys -dgput`: about 7.7 ms and 146 `cl100k_base` tokens on the benchmark machine.
  - all-fields output uses less than half the tokens of every comparison output captured in the release run.

### Removed

- Subprocess dependency on `df` for disk usage.
- Subprocess dependency on `lspci` for GPU information.
- Older README benchmark tables from prior measurements.

---

## v1.0.0 — 2026-04-10

Initial public release.

### Added

- Always-on base fields: `os`, `host`, `kernel`, `shell`, `cpu`, `memory`.
- Optional fields:
  - `-d` disk usage
  - `-g` GPU
  - `-p` packages
  - `-u` uptime
  - `-t` elapsed execution time
- Linux release artifacts for x86_64, aarch64, and armv7.
