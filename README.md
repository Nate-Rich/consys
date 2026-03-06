# consys

Concise system information. No logos, no fluff.

```
os:       Arch Linux
host:     <machine model>
kernel:   6.x.x-arch1-1
shell:    /bin/bash
cpu:      <processor model>
memory:   xxxxM / xxxxM
```

Written in Rust. Reads directly from `/proc` and `/sys` where possible — no unnecessary subprocesses.

---

## Install

### Option 1: Secure (recommended)

Download the install script, review it, then run it:

```bash
curl -o /tmp/install.sh https://raw.githubusercontent.com/Nate-Rich/consys/main/install.sh
```

Review: `cat /tmp/install.sh`

```bash
bash /tmp/install.sh
```

Verifies SHA256 checksum before installing. Aborts if verification fails.

### Option 2: Quick

```bash
bash <(curl -s https://raw.githubusercontent.com/Nate-Rich/consys/main/install.sh)
```

Both options detect your architecture automatically and install to `/usr/local/bin/consys`.

### Option 3: Manual binary

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

### Option 4: Build from source

Requires [Rust](https://rustup.rs).

```bash
git clone https://github.com/Nate-Rich/consys.git
cd consys
cargo build --release
sudo cp target/release/consys /usr/local/bin/consys
```

---

## Usage

```
consys [flags]
```

All flags are optional. With no flags, consys prints os, host, kernel, shell, cpu, and memory.

| Flag | Field | Notes |
|------|-------|-------|
| `-d` | disk | root partition `/` only |
| `-g` | gpu | requires `lspci` (`pciutils`) |
| `-p` | pkgs | dpkg, pacman, flatpak, and/or snap |
| `-u` | uptime | |
| `-t` | time | time taken to fetch |
| `--help` | help | |
| `--version` | version | |

### Output order

When flags are supplied, output always follows this order regardless of how flags are passed:

```
os
host
kernel
shell
cpu
memory
disk      (-d)
gpu       (-g)
pkgs      (-p)
uptime    (-u)
time      (-t)
```

### Example

```bash
consys -g -d -p -u -t
```

---

## Uninstall

```bash
sudo rm /usr/local/bin/consys
```

---

## Notes

- GPU detection requires `lspci`, part of `pciutils` — standard on most Linux distros
- Disk usage reflects the root partition `/` only
- Package count covers dpkg, pacman, flatpak, and snap where present
- No configuration, no dependencies, no logos
