//! consys — concise system information.
//!
//! Each `write_*` function follows the same pattern:
//!   a) read a file (or call a libc fn)
//!   b) parse into a small slice or number
//!   c) write a `key: value` line directly to the shared writer
//!
//! `<W: Write>` is a generic parameter — at compile time, Rust generates a
//! specialized version of each function for the concrete writer type we hand
//! it (`BufWriter<StdoutLock>`). No vtables, no dynamic dispatch.

use std::fs;
use std::io::{self, BufWriter, Write};

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ─── always-on fields ─────────────────────────────────────────────────────────
//
// These six fields print on every invocation. Each runs in ~50–200 µs.
// Pattern: read one file, find one line, write one output line.

/// OS pretty name from /etc/os-release, e.g. "Ubuntu 24.04 LTS".
fn write_os<W: Write>(w: &mut W) -> io::Result<()> {
    let s = fs::read_to_string("/etc/os-release").unwrap_or_default();
    // find_map = "find + map combined": run the closure on each line; the first
    // one that returns Some(x) wins. strip_prefix returns Some(rest) on match.
    let val = s
        .lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"'))
        .unwrap_or("unknown");
    writeln!(w, "os:       {val}")
}

/// Machine name from DMI tables exposed under /sys/devices/virtual/dmi/id/.
/// Many machines have product_version empty or duplicating product_name —
/// both edge cases are handled.
fn write_host<W: Write>(w: &mut W) -> io::Result<()> {
    let name = fs::read_to_string("/sys/devices/virtual/dmi/id/product_name").unwrap_or_default();
    let name = name.trim(); // shadowing: rebind `name` to the trimmed slice
    if name.is_empty() {
        return writeln!(w, "host:     unknown");
    }
    let ver = fs::read_to_string("/sys/devices/virtual/dmi/id/product_version").unwrap_or_default();
    let ver = ver.trim();
    if ver.is_empty() || ver == name {
        writeln!(w, "host:     {name}")
    } else {
        writeln!(w, "host:     {name} {ver}")
    }
}

/// Kernel version from /proc/version. The third whitespace token is what we want:
///   "Linux version 6.8.0-31-generic (...) ..."
fn write_kernel<W: Write>(w: &mut W) -> io::Result<()> {
    let s = fs::read_to_string("/proc/version").unwrap_or_default();
    let val = s.split_whitespace().nth(2).unwrap_or("unknown");
    writeln!(w, "kernel:   {val}")
}

/// Login shell from $SHELL.
fn write_shell<W: Write>(w: &mut W) -> io::Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "unknown".into());
    writeln!(w, "shell:    {shell}")
}

/// CPU model from /proc/cpuinfo. /proc/cpuinfo has one block per logical core,
/// all with identical "model name" lines on homogeneous CPUs — we take the first.
/// `find` short-circuits on first hit, so we don't scan the whole (large) file.
fn write_cpu<W: Write>(w: &mut W) -> io::Result<()> {
    let s = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let val = s
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':')) // "model name : Intel..." → ("model name ", " Intel...")
        .map(|(_, v)| v.trim()) // ignore key half, trim value half
        .unwrap_or("unknown");
    writeln!(w, "cpu:      {val}")
}

/// Memory: used / total in MiB, parsed from /proc/meminfo.
///
/// "used" = MemTotal − MemAvailable (matches `free -h`'s definition; more
/// accurate than Total − Free, which doesn't account for reclaimable cache).
fn write_memory<W: Write>(w: &mut W) -> io::Result<()> {
    let s = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64;
    let mut avail = 0u64;
    for line in s.lines() {
        // let-else: try to destructure; on None, run the else block (must diverge).
        // After this line, `key` and `rest` are normal bindings.
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        match key {
            "MemTotal" => {
                total = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
            }
            "MemAvailable" => {
                avail = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
            }
            _ => continue,
        }
        // Bail as soon as both numbers are populated — meminfo has 50+ lines we don't need.
        if total != 0 && avail != 0 {
            break;
        }
    }
    // saturating_sub returns 0 instead of wrapping/panicking on underflow.
    let used = total.saturating_sub(avail) / 1024; // KiB → MiB
    let total = total / 1024;
    writeln!(w, "memory:   {used}M / {total}M")
}

// ─── opt-in fields ────────────────────────────────────────────────────────────
//
// Below are the flag-gated features. They're heavier — disk uses libc FFI,
// gpu walks /sys/bus/pci, packages stats multiple package-manager databases,
// time uses libc localtime_r — so they only run when explicitly requested.

/// Disk usage of "/" via libc statvfs64.
///
/// Why libc instead of spawning `df`? A subprocess (fork+exec+glibc startup)
/// is ~5–15 ms; statvfs64 is one syscall, ~50 µs. Order of magnitude faster.
///
/// Why statvfs64 specifically (not statvfs)?
///   - statvfs's struct fields use C `unsigned long`, which is 32-bit on
///     armv7 and 64-bit on aarch64/x86_64. Different layouts per target.
///   - statvfs64 has uniform u64 fields on every Linux glibc target.
///     One layout, no #[cfg] gymnastics, no surprise UB on ARM.
fn write_disk<W: Write>(w: &mut W) -> io::Result<()> {
    // Static nul-terminated C path avoids constructing a CString (which would
    // heap-allocate). c_char varies signedness by target, so spell the bytes
    // with libc::c_char rather than casting a u8 pointer.
    const ROOT: [libc::c_char; 2] = [b'/' as libc::c_char, 0];
    let path = ROOT.as_ptr();

    // FFI is unsafe: the compiler cannot verify C functions respect Rust's
    // invariants. Safety conditions met here:
    //   - `path` points to a valid null-terminated C string ✓
    //   - `&mut s` points to a valid statvfs64-sized writable region ✓
    //   - mem::zeroed is safe for statvfs64 (only integer fields, no refs) ✓
    unsafe {
        let mut s: libc::statvfs64 = std::mem::zeroed();
        if libc::statvfs64(path, &mut s) == 0 {
            // We use f_bfree (true free space), NOT f_bavail (free space
            // available to non-root users). On ext4 the default 5% root
            // reservation makes those values differ; for a "is the disk full"
            // question we want the partition-level answer, not the
            // unprivileged-user answer.
            let total = bytes_to_gib(s.f_blocks as u64, s.f_frsize as u64);
            let free = bytes_to_gib(s.f_bfree as u64, s.f_frsize as u64);
            let used = total.saturating_sub(free);
            return writeln!(w, "disk:     {used}G / {total}G");
        }
    }
    writeln!(w, "disk:     unknown")
}

/// Convert (block_count, fragment_size) to GiB, using u128 internally to
/// guard against overflow. u64 multiplication overflows at ~16 EiB worth of
/// small fragments — real filesystems aren't there yet, but the cost of u128
/// is negligible (one extra mul instruction).
fn bytes_to_gib(blocks: u64, frsize: u64) -> u64 {
    (((blocks as u128) * (frsize as u128)) >> 30) as u64
}

/// Look up a (vendor_id, device_id) pair in the system's pci.ids database.
/// Returns Some((vendor_name, device_name)) on hit, None otherwise.
///
/// pci.ids format:
///   8086  Intel Corporation         ← vendor (no leading whitespace)
///   \t1234  Some Device             ← device under previous vendor (one tab)
///   \t\t10de 1234 Some Subdevice    ← subsystem (two tabs, ignored)
///   1002  Advanced Micro Devices    ← next vendor block starts
///
/// The file is sorted by vendor ID, then device ID within each vendor block,
/// so we can short-circuit as soon as we leave our vendor's block.
fn read_pci_ids(vendor_id: &str, device_id: &str) -> Option<(String, String)> {
    use std::io::BufRead;

    // Find the first existing pci.ids location. Trailing `?` returns None
    // from the whole function if neither exists.
    let path = ["/usr/share/misc/pci.ids", "/usr/share/hwdata/pci.ids"]
        .iter()
        .find(|p| std::path::Path::new(p).exists())?;

    // pci.ids is ~1 MB. Don't slurp it all into memory — stream line by line.
    let file = fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);

    let mut vendor_name: Option<String> = None;

    for line in reader.lines().map_while(Result::ok) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if !line.starts_with('\t') {
            // Top-level line = a vendor entry.
            // If we already matched our vendor, hitting another vendor means
            // we've left our block without finding the device — bail.
            if vendor_name.is_some() {
                return None;
            }
            if let Some(rest) = line.strip_prefix(vendor_id) {
                // .to_string() materializes a String we can hold across iterations
                // (the `line` itself is dropped each iteration).
                vendor_name = Some(rest.trim_start().to_string());
            }
        } else if vendor_name.is_some() && !line.starts_with("\t\t") {
            // Single-tab line + matched vendor = device entry.
            // (Double-tab lines are subsystem entries; we ignore them.)
            let trimmed = &line[1..]; // skip the leading \t
            if let Some(rest) = trimmed.strip_prefix(device_id) {
                return Some((vendor_name?, rest.trim_start().to_string()));
            }
        }
    }
    None
}

/// GPU info via /sys/bus/pci walk + pci.ids lookup.
///
/// Strategy:
///   1. Find all PCI devices with class 0x0300xx (display) or 0x0380xx (3D).
///   2. Sort by PCI address for stable, bus-order display.
///   3. For each, try pci.ids for human-readable names; fall back to IDs.
///   4. Single GPU → "gpu:" key. Multi-GPU → "gpu0:", "gpu1:", ...
///
/// Why indexed keys for multi-GPU: repeated "gpu:" lines break naive
/// last-write-wins parsers. Indexed keys preserve all entries uniquely.
///
/// Cost: ~2–4 ms with pci.ids hit, ~250 µs without. Both massively cheaper
/// than spawning lspci (~15 ms for fork+exec alone).
fn write_gpu<W: Write>(w: &mut W) -> io::Result<()> {
    let dir = match fs::read_dir("/sys/bus/pci/devices") {
        Ok(e) => e,
        Err(_) => return writeln!(w, "gpu:      unknown"),
    };

    // Collect first, sort second — read_dir order is NOT stable across boots
    // or between machines. We need a deterministic display order.
    let mut gpus: Vec<(String, String, String)> = Vec::new(); // (pci_addr, vid, did)

    for entry in dir.flatten() {
        let path = entry.path();
        let class = fs::read_to_string(path.join("class")).unwrap_or_default();
        let class = class.trim();

        // PCI class code is 0xCCSSPP: CC=class, SS=subclass, PP=programming interface.
        // 0x0300xx = Display controller (most GPUs)
        // 0x0380xx = "Other display controller" (some compute-only / 3D-only cards)
        if !(class.starts_with("0x0300") || class.starts_with("0x0380")) {
            continue;
        }

        let vendor = fs::read_to_string(path.join("vendor")).unwrap_or_default();
        let device = fs::read_to_string(path.join("device")).unwrap_or_default();
        // sysfs reports "0x8086\n" — strip both prefix and whitespace.
        let vid = vendor.trim().trim_start_matches("0x").to_string();
        let did = device.trim().trim_start_matches("0x").to_string();

        // Entry filename is "DDDD:BB:DD.F" (domain:bus:device.function), fixed-width.
        // Lex-sort on these strings IS PCI bus order — no parsing needed.
        let addr = entry.file_name().to_string_lossy().into_owned();
        gpus.push((addr, vid, did));
    }

    if gpus.is_empty() {
        return writeln!(w, "gpu:      unknown");
    }
    gpus.sort_by(|a, b| a.0.cmp(&b.0));

    let multi = gpus.len() > 1;

    for (i, (_addr, vid, did)) in gpus.iter().enumerate() {
        // {key:<10} pads `key` to 10 chars with trailing spaces — matches the
        // file-wide column convention ("os:       ", "kernel:   ", etc.).
        let key = if multi {
            format!("gpu{i}:")
        } else {
            "gpu:".into()
        };

        if let Some((vname, dname)) = read_pci_ids(vid, did) {
            // Strip noise: most IHV strings end in " Corporation" or " Inc."
            let vname = vname
                .trim_end_matches(" Corporation")
                .trim_end_matches(" Inc.");
            writeln!(w, "{key:<10}{vname} {dname}")?;
        } else {
            // Graceful fallback: pci.ids missing or device unknown.
            // Output is still useful for log/agent consumers as machine-parseable IDs.
            let fallback = match vid.as_str() {
                "10de" => "NVIDIA",
                "1002" => "AMD",
                "8086" => "Intel",
                _ => "Unknown",
            };
            writeln!(w, "{key:<10}{fallback} [0x{vid}:0x{did}]")?;
        }
    }
    Ok(())
}

/// Count actually-installed dpkg packages (not residual config-files entries).
///
/// /var/lib/dpkg/status has one block per package known to dpkg. A block looks like:
///
///   Package: bash
///   Status: install ok installed     ← present and working
///   ...
///   <blank line>
///
/// Removed-but-config-retained packages also have a Package: line:
///
///   Package: foo
///   Status: deinstall ok config-files
///
/// Counting all Package: lines overcounts. We track block boundaries (blank
/// lines) and only count blocks whose Status ends in " installed".
fn count_dpkg_installed() -> usize {
    let s = fs::read_to_string("/var/lib/dpkg/status").unwrap_or_default();
    let mut count = 0usize;
    let mut in_pkg = false;
    let mut installed = false;

    for line in s.lines() {
        if line.is_empty() {
            // End of a package block: tally if it qualified.
            if in_pkg && installed {
                count += 1;
            }
            in_pkg = false;
            installed = false;
        } else if line.starts_with("Package:") {
            in_pkg = true;
        } else if line.starts_with("Status:") {
            // "install ok installed" is the only status string ending in
            // " installed". Other variants: "deinstall ok config-files",
            // "purge ok not-installed", "install ok unpacked", etc.
            installed = line.ends_with(" installed");
        }
    }
    // Catch a trailing block that wasn't followed by a blank line.
    if in_pkg && installed {
        count += 1;
    }
    count
}

/// Count installed snaps via /snap/<name>/current symlinks (one per app),
/// not via .snap blob files (snapd retains old revisions).
fn count_snap_installed() -> usize {
    fs::read_dir("/snap/")
        .map(|d| {
            d.flatten()
                .filter(|e| {
                    let p = e.path();
                    p.is_dir() && p.join("current").exists()
                })
                .count()
        })
        .unwrap_or(0)
}

/// Count entries (one per installed package) under a given dir.
fn count_dirs(path: &str) -> usize {
    fs::read_dir(path)
        .map(|d| d.flatten().filter(|e| e.path().is_dir()).count())
        .unwrap_or(0)
}

/// Package counts across detected package managers.
/// Output: "pkgs:     2426 (dpkg), 5 (flatpak)" or similar.
///
/// Only present package managers contribute. If none are present, prints "unknown".
fn write_packages<W: Write>(w: &mut W) -> io::Result<()> {
    let dpkg = count_dpkg_installed();
    let pacman = count_dirs("/var/lib/pacman/local/");
    let flatpak = {
        // Both system-wide and per-user installations exist on flatpak.
        let sys = count_dirs("/var/lib/flatpak/app/");
        let home = std::env::var("HOME").unwrap_or_default();
        let user = count_dirs(&format!("{home}/.local/share/flatpak/app/"));
        sys + user
    };
    let snap = count_snap_installed();

    write!(w, "pkgs:     ")?;
    let mut first = true;
    for (n, name) in [
        (dpkg, "dpkg"),
        (pacman, "pacman"),
        (flatpak, "flatpak"),
        (snap, "snap"),
    ] {
        if n == 0 {
            continue;
        }
        if !first {
            write!(w, ", ")?;
        }
        write!(w, "{n} ({name})")?;
        first = false;
    }
    if first {
        write!(w, "unknown")?;
    }
    writeln!(w)
}

/// Uptime as "Hh Mm" (or "Mm" if under an hour).
/// /proc/uptime: "12345.67 67890.12" — first field is seconds since boot.
fn write_uptime<W: Write>(w: &mut W) -> io::Result<()> {
    let s = fs::read_to_string("/proc/uptime").unwrap_or_default();
    let secs = s
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        writeln!(w, "uptime:   {h}h {m}m")
    } else {
        writeln!(w, "uptime:   {m}m")
    }
}

/// Local time as "YYYY-MM-DD HH:MM:SS UTC±HH:MM".
/// Human-readable format based on ISO 8601 (relaxed separator: space, not 'T').
///
/// Why libc::tm and libc::localtime_r: Rust's stdlib has SystemTime but no
/// timezone-aware formatting. We need the kernel's notion of local time and
/// the system's TZ rules — that's what localtime_r does. Going through libc
/// avoids re-implementing the IANA zoneinfo lookup ourselves.
fn write_time<W: Write>(w: &mut W) -> io::Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // libc::time_t varies per target (32 or 64 bits depending on Y2038 settings);
    // casting through it ensures correctness on every supported platform.
    let secs: libc::time_t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as libc::time_t;

    unsafe {
        // mem::zeroed is safe for libc::tm: all fields are integers/pointers
        // that accept zero-initialization.
        let mut tm: libc::tm = std::mem::zeroed();
        // localtime_r returns NULL on error, otherwise the same pointer it was given.
        if libc::localtime_r(&secs, &mut tm).is_null() {
            return writeln!(w, "time:     unknown");
        }

        // Decompose tm_gmtoff (signed seconds offset) into ±HH:MM.
        // .abs() before % handles negative offsets (e.g. UTC-06:00) correctly.
        let off_h = tm.tm_gmtoff / 3600;
        let off_m = (tm.tm_gmtoff.abs() % 3600) / 60;
        let sign = if tm.tm_gmtoff >= 0 { '+' } else { '-' };

        // tm_year is years-since-1900, tm_mon is 0-indexed → +1900, +1.
        // {:04} / {:02} = zero-pad to 4 / 2 digits.
        writeln!(
            w,
            "time:     {:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC{}{:02}:{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec,
            sign,
            off_h.abs(),
            off_m,
        )
    }
}

// ─── CLI ──────────────────────────────────────────────────────────────────────

fn print_help() {
    println!("consys - concise system information\n");
    println!("usage:  consys [flags]\n");
    println!("flags:");
    println!("  -d         disk");
    println!("  -g         gpu");
    println!("  -p         packages");
    println!("  -u         uptime");
    println!("  -t         time");
    println!("  --help     help");
    println!("  --version  version\n");
    println!("example:");
    println!("  consys -g -d -u -p -t");
}

/// Entry point. Parse args, set flags, write all enabled fields through one
/// shared buffered writer.
///
/// Returns io::Result<()> so `?` can propagate write failures (broken pipe,
/// disk full, etc.) as a clean non-zero exit.
fn main() -> io::Result<()> {
    let (mut d, mut g, mut p, mut u, mut t) = (false, false, false, false, false);

    // env::args() yields the program name first; .skip(1) drops it.
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--help" => {
                print_help();
                return Ok(());
            }
            "--version" => {
                println!("consys {VERSION}");
                return Ok(());
            }

            // Match guard: pattern + condition. Catches "-d", "-gp", "-dgput", etc.
            // Each char after '-' is a separate flag, so "-gd" == "-g -d".
            a if a.starts_with('-') => {
                for c in a.chars().skip(1) {
                    match c {
                        'd' => d = true,
                        'g' => g = true,
                        'p' => p = true,
                        'u' => u = true,
                        't' => t = true,
                        // Strict mode: unknown flags fail loudly with exit 2.
                        // Silent acceptance of typos turns "consys --gpuz" into
                        // a "successful" base run — bad for automation.
                        other => {
                            eprintln!("consys: unknown flag '-{other}'");
                            eprintln!("Try 'consys --help' for usage.");
                            std::process::exit(2);
                        }
                    }
                }
            }
            // Non-flag positional args: silently dropped (we have none).
            _ => {}
        }
    }

    // === The performance trick at the heart of the program ===
    //
    // Every `println!` would take the global stdout lock, write, and release
    // the lock — that's a syscall + lock-acquire per line.
    //
    // Instead: acquire the lock ONCE, wrap a 512-byte BufWriter around it.
    // All writes accumulate in memory. When `w` goes out of scope its Drop
    // impl flushes — but Drop swallows errors, so we flush explicitly first
    // to surface broken-pipe and similar conditions.
    let stdout = io::stdout();
    let mut w = BufWriter::with_capacity(512, stdout.lock());

    // Always-on fields, in display order.
    write_os(&mut w)?;
    write_host(&mut w)?;
    write_kernel(&mut w)?;
    write_shell(&mut w)?;
    write_cpu(&mut w)?;
    write_memory(&mut w)?;

    // Opt-in fields, canonical order.
    if d {
        write_disk(&mut w)?;
    }
    if g {
        write_gpu(&mut w)?;
    }
    if p {
        write_packages(&mut w)?;
    }
    if u {
        write_uptime(&mut w)?;
    }
    if t {
        write_time(&mut w)?;
    }

    // Explicit flush surfaces errors that Drop would swallow.
    w.flush()?;
    Ok(())
}
