# consys

Concise system information. No logos, no fluff.

```
os:     Ubuntu 24.04 LTS
kernel: 6.8.0-101-generic
cpu:    Intel(R) Core(TM) i7-9750H CPU @ 2.60GHz
memory: 4821M / 15926M
```

Written in Rust. Reads directly from `/proc` and `/sys` where possible — no unnecessary subprocesses.

---

## Install

### Option 1: Script (recommended)

```bash
bash <(curl -s https://raw.githubusercontent.com/Nate-Rich/consys/main/install.sh)
```

Detects your architecture, downloads the correct binary from releases, installs to `/usr/local/bin/consys`.

### Option 2: Manual binary

Download the correct binary for your architecture from [releases](https://github.com/Nate-Rich/consys/releases/latest):

| Architecture | Binary |
|---|---|
| x86_64 (most desktops/laptops) | `consys-x86_64-unknown-linux-gnu` |
| aarch64 (Raspberry Pi 4, ARM servers) | `consys-aarch64-unknown-linux-gnu` |
| armv7 (older ARM devices) | `consys-armv7-unknown-linux-gnueabihf` |

```bash
sudo mv consys-* /usr/local/bin/consys
sudo chmod +x /usr/local/bin/consys
```

### Option 3: Build from source

Requires [Rust](https://rustup.rs).

```bash
git clone https://github.com/Nate-Rich/consys.git
cd consys
cargo build --release
sudo cp target/release/consys /usr/local/bin/consys
```

---

## Use

consys - concise system information

usage:  consys [flags]

flags:
  -g    gpu
  -d    disk
  -u    uptime
  -p    packages
  -f    fetchtime
  -h    help

example:
  consys -g -d -u -p -f

---

## Uninstall

```bash
sudo rm /usr/local/bin/consys
```

---

## Notes

- GPU detection requires `lspci` (part of `pci-utils`, standard on most Linux distros)
- Disk usage reflects the root partition `/` only
- No configuration, no dependencies, no logos