use std::fs;
use std::process::Command;
use std::time::Instant;

fn os() -> String {
    let content = fs::read_to_string("/etc/os-release").unwrap_or_default();
    for line in content.lines() {
        if line.starts_with("PRETTY_NAME=") {
            return line[12..].trim_matches('"').to_string();
        }
    }
    "unknown".to_string()
}

fn host() -> String {
    let name = fs::read_to_string("/sys/devices/virtual/dmi/id/product_name")
        .unwrap_or_default().trim().to_string();
    let version = fs::read_to_string("/sys/devices/virtual/dmi/id/product_version")
        .unwrap_or_default().trim().to_string();
    if name.is_empty() { return "unknown".to_string(); }
    if version.is_empty() || version == name { return name; }
    format!("{} {}", name, version)
}

fn kernel() -> String {
    fs::read_to_string("/proc/version")
        .unwrap_or_default()
        .split_whitespace()
        .nth(2)
        .unwrap_or("unknown")
        .to_string()
}

fn shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string())
}

fn cpu() -> String {
    let content = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    for line in content.lines() {
        if line.starts_with("model name") {
            if let Some(val) = line.splitn(2, ':').nth(1) {
                return val.trim().to_string();
            }
        }
    }
    "unknown".to_string()
}

fn gpu() -> String {
    let output = match Command::new("lspci").output() {
        Ok(o) => o,
        Err(_) => return "unknown".to_string(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if lower.contains("vga") || lower.contains("display") {
            if let Some(after_first) = line.splitn(2, ':').nth(1) {
                if let Some(after_second) = after_first.splitn(2, ':').nth(1) {
                    let trimmed = after_second.trim();
                    let clean = if let Some(idx) = trimmed.find(" (rev ") {
                        &trimmed[..idx]
                    } else {
                        trimmed
                    };
                    return clean.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

fn disk() -> String {
    let output = match Command::new("df").args(["-h", "/"]).output() {
        Ok(o) => o,
        Err(_) => return "unknown".to_string(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().nth(1)
        .and_then(|l| {
            let p: Vec<&str> = l.split_whitespace().collect();
            if p.len() >= 3 { Some(format!("{} / {}", p[2], p[1])) } else { None }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn memory() -> String {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total_kb = 0u64;
    let mut available_kb = 0u64;
    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = line.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            available_kb = line.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        }
    }
    let used = (total_kb - available_kb) / 1024;
    let total = total_kb / 1024;
    format!("{}M / {}M", used, total)
}

fn pkgs_dpkg() -> usize {
    let content = fs::read_to_string("/var/lib/dpkg/status").unwrap_or_default();
    content.lines().filter(|l| l.starts_with("Package:")).count()
}

fn pkgs_pacman() -> usize {
    fs::read_dir("/var/lib/pacman/local/")
        .map(|d| d.count())
        .unwrap_or(0)
}

fn pkgs_flatpak() -> usize {
    let sys = fs::read_dir("/var/lib/flatpak/app/")
        .map(|d| d.count())
        .unwrap_or(0);
    let user = fs::read_dir(
        std::env::var("HOME").unwrap_or_default() + "/.local/share/flatpak/app/"
    )
    .map(|d| d.count())
    .unwrap_or(0);
    sys + user
}

fn pkgs_snap() -> usize {
    fs::read_dir("/var/lib/snapd/snaps/")
        .map(|d| d
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "snap").unwrap_or(false))
            .count())
        .unwrap_or(0)
}

fn packages() -> String {
    let dpkg    = pkgs_dpkg();
    let pacman  = pkgs_pacman();
    let flatpak = pkgs_flatpak();
    let snap    = pkgs_snap();

    let mut parts = Vec::new();
    if dpkg    > 0 { parts.push(format!("{} (dpkg)",    dpkg));    }
    if pacman  > 0 { parts.push(format!("{} (pacman)",  pacman));  }
    if flatpak > 0 { parts.push(format!("{} (flatpak)", flatpak)); }
    if snap    > 0 { parts.push(format!("{} (snap)",    snap));    }

    if parts.is_empty() { "unknown".to_string() } else { parts.join(", ") }
}

fn uptime() -> String {
    let content = fs::read_to_string("/proc/uptime").unwrap_or_default();
    let secs = content.split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{}h {}m", h, m) } else { format!("{}m", m) }
}

fn help() {
    println!("consys - concise system information");
    println!();
    println!("usage:  consys [flags]");
    println!();
    println!("flags:");
    println!("  -g    gpu");
    println!("  -d    disk");
    println!("  -u    uptime");
    println!("  -p    packages");
    println!("  -t    time to fetch");
    println!("  -h    help");
    println!();
    println!("example:");
    println!("  consys -g -d -u -p -t");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "-h" || a == "--help" || a == "help") {
        help();
        return;
    }

    let show_gpu      = args.iter().any(|a| a == "-g");
    let show_disk     = args.iter().any(|a| a == "-d");
    let show_uptime   = args.iter().any(|a| a == "-u");
    let show_packages = args.iter().any(|a| a == "-p");
    let show_time     = args.iter().any(|a| a == "-t");

    let start = Instant::now();

    println!("os:       {}", os());
    println!("host:     {}", host());
    println!("kernel:   {}", kernel());
    println!("shell:    {}", shell());
    println!("cpu:      {}", cpu());
    if show_gpu      { println!("gpu:      {}", gpu()); }
    if show_disk     { println!("disk:     {}", disk()); }
    println!("memory:   {}", memory());
    if show_packages { println!("pkgs:     {}", packages()); }
    if show_uptime   { println!("uptime:   {}", uptime()); }
    if show_time     { println!("time:     {:?}", start.elapsed()); }
}