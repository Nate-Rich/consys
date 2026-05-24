# consys

Concise system information for the multi-user environment.

A developer, someone using text-to-speech, a language model or agent, a monitoring script, and a log pipeline may all need the same machine context. Most system fetch tools were built for one reader: a human who wants a pretty terminal. consys is built for everyone.

Clean `key: value` output. No art. No decoration. A small, stable snapshot that is readable by humans and easy for machines to parse.

## Output reference

```text
os:       <OS pretty name>
host:     <machine model>
kernel:   <kernel release>
shell:    <login shell>
cpu:      <processor model>
memory:   <used>M / <total>M
disk:     <used>G / <total>G              # -d
gpu:      <gpu model>                     # -g, single GPU
gpu0:     <gpu model>                     # -g, multi-GPU
gpu1:     <gpu model>                     # -g, multi-GPU
pkgs:     <n> (dpkg), <n> (flatpak)       # -p
uptime:   <h>h <m>m                       # -u
time:     YYYY-MM-DD HH:MM:SS UTC±HH:MM   # -t
```

If a field cannot be read, consys prints `unknown` where practical instead of failing the whole command. Output field names and order are treated as a compatibility contract.

---

## Flags

```text
consys [flags]

  -d         disk usage
  -g         GPU
  -p         packages (dpkg, pacman, flatpak, snap)
  -u         uptime
  -t         local time

  --help     help
  --version  version
```

Base output is always on: `os`, `host`, `kernel`, `shell`, `cpu`, `memory`.

Optional fields are appended in canonical order no matter how flags are written. These are equivalent:

```sh
consys -d -g -p -u -t
consys -dgput
```

---

## Design notes

- Linux only.
- No configuration file.
- No colors, icons, logos, or ASCII art.
- One small dependency: `libc`, used for correct Linux FFI bindings.
- Disk usage is read with `statvfs64`, not by spawning `df`.
- GPU information is read from sysfs and `pci.ids`, not by spawning `lspci`.
- Package counts are read directly from package-manager state.

consys is intentionally boring. The output is the product.

---

## Benchmarks

The benchmark process is documented in [BENCHMARKS.md](BENCHMARKS.md).

On the v1.1.0 benchmark machine, base `consys` output ran in about 0.8 ms warm-cache and used 79 `cl100k_base` tokens. All-fields `consys -dgput` output ran in about 7.7 ms and used 146 tokens.

The full benchmark notes include environment details, comparison-tool timings, and output-size tables.

---

## Install

### Option 1: installer script

This is the shortest path. It detects your Linux architecture, downloads the latest release archive, verifies the archive checksum, and installs `consys` to `/usr/local/bin/consys`.

Read the script first if you want to see exactly what it does:

```sh
curl -fsSL https://raw.githubusercontent.com/Nate-Rich/consys/main/install.sh
```

Then run it:

```sh
curl -fsSL https://raw.githubusercontent.com/Nate-Rich/consys/main/install.sh | bash
```

### Option 2: manual release install

Manual install is only a few steps. Keep them separate so you can see what each one does.

#### 1. Choose your architecture

```sh
uname -m
```

Use the matching release asset:

| `uname -m` | Asset |
|:--|:--|
| `x86_64` | `consys-v1.1.0-x86_64-unknown-linux-gnu.tar.gz` |
| `aarch64` | `consys-v1.1.0-aarch64-unknown-linux-gnu.tar.gz` |
| `armv7l` | `consys-v1.1.0-armv7-unknown-linux-gnueabihf.tar.gz` |

The examples below use x86_64 and v1.1.0. Replace the asset name and version if you are installing a different release.

#### 2. Download the archive

```sh
curl -LO https://github.com/Nate-Rich/consys/releases/download/v1.1.0/consys-v1.1.0-x86_64-unknown-linux-gnu.tar.gz
```

#### 3. Check the SHA-256 sum

Verification is separate from installation on purpose. It confirms that the file you downloaded is the same file that was published in the GitHub release.

Low-friction check: GitHub release notes list the SHA-256 sums. Calculate the local sum and compare it with the value shown on the release page:

```sh
sha256sum consys-v1.1.0-x86_64-unknown-linux-gnu.tar.gz
```

Scriptable check: download `sha256.txt` from the same release and ask `sha256sum` to verify only the file you downloaded:

```sh
curl -LO https://github.com/Nate-Rich/consys/releases/download/v1.1.0/sha256.txt
grep 'consys-v1.1.0-x86_64-unknown-linux-gnu.tar.gz$' sha256.txt | sha256sum --check
```

Do not install the binary if the checksum does not match.

#### 4. Unpack and install

```sh
tar -xzf consys-v1.1.0-x86_64-unknown-linux-gnu.tar.gz
sudo install -m 755 consys /usr/local/bin/consys
consys --version
```

### Option 3: build from source

```sh
git clone https://github.com/Nate-Rich/consys
cd consys
cargo build --release
sudo install -m 755 target/release/consys /usr/local/bin/consys
consys --version
```

Requires Rust stable.

---

## Upgrade

### Upgrade with the installer script

```sh
curl -fsSL https://raw.githubusercontent.com/Nate-Rich/consys/main/install.sh | bash
consys --version
```

### Upgrade manually

1. Download the newer release archive for your architecture.
2. Verify its SHA-256 sum.
3. Unpack it.
4. Replace the existing binary:

```sh
sudo install -m 755 consys /usr/local/bin/consys
consys --version
```

### Upgrade from source

```sh
cd consys
git pull --ff-only
cargo build --release
sudo install -m 755 target/release/consys /usr/local/bin/consys
consys --version
```

---

## Uninstall

```sh
sudo rm /usr/local/bin/consys
```

---

## License

MIT — [github.com/Nate-Rich/consys](https://github.com/Nate-Rich/consys)
