//! Terminal styling (ANSI, disabled for NO_COLOR or non-terminals).

use std::io::IsTerminal;
use std::sync::OnceLock;

pub fn color() -> bool {
    static C: OnceLock<bool> = OnceLock::new();
    *C.get_or_init(|| {
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        let forced = ["FORCE_COLOR", "CLICOLOR_FORCE"].iter().any(|k| std::env::var(k).is_ok_and(|v| v != "0"));
        forced || std::io::stderr().is_terminal()
    })
}

fn paint(code: &str, s: &str) -> String {
    if color() { format!("\x1b[{code}m{s}\x1b[0m") } else { s.to_owned() }
}

pub fn bold(s: &str) -> String {
    paint("1", s)
}
pub fn dim(s: &str) -> String {
    paint("2", s)
}
pub fn red(s: &str) -> String {
    paint("31", s)
}
pub fn green(s: &str) -> String {
    paint("32", s)
}
pub fn yellow(s: &str) -> String {
    paint("33", s)
}
pub fn cyan(s: &str) -> String {
    paint("36", s)
}
pub fn magenta(s: &str) -> String {
    paint("35", s)
}

pub fn step(msg: &str) {
    eprintln!("{} {msg}", cyan("▲"));
}

pub fn ok(msg: &str) {
    eprintln!("{} {msg}", green("✓"));
}

pub fn warn(msg: &str) {
    eprintln!("{} {msg}", yellow("⚠"));
}

pub fn fail(msg: &str) {
    eprintln!("{} {msg}", red("✗"));
}

/// Pad accounting for ANSI escape sequences.
pub fn pad(s: &str, width: usize) -> String {
    let visible = strip_ansi(s).chars().count();
    format!("{s}{}", " ".repeat(width.saturating_sub(visible)))
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        match (in_escape, c) {
            (false, '\x1b') => in_escape = true,
            (true, 'm') => in_escape = false,
            (true, _) => {}
            (false, c) => out.push(c),
        }
    }
    out
}

pub fn bytes(n: u64) -> String {
    match n {
        0..1024 => format!("{n} B"),
        1024..1_048_576 => format!("{:.1} kB", n as f64 / 1024.0),
        _ => format!("{:.1} MB", n as f64 / 1_048_576.0),
    }
}

// ---------------------------------------------------------------------------
// Brand visuals
// ---------------------------------------------------------------------------

/// Whether the terminal advertises 24-bit colour.
fn truecolor() -> bool {
    std::env::var("COLORTERM").is_ok_and(|v| v.contains("truecolor") || v.contains("24bit"))
}

/// Foreground colour escape for an RGB value (24-bit, or the nearest of the
/// 256-colour palette on terminals like macOS Terminal.app).
fn fg(r: u8, g: u8, b: u8) -> String {
    if truecolor() {
        format!("\x1b[38;2;{r};{g};{b}m")
    } else {
        let q = |v: u8| (u16::from(v) * 5 / 255) as u8;
        format!("\x1b[38;5;{}m", 16 + 36 * q(r) + 6 * q(g) + q(b))
    }
}

fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    (mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

/// Paint `text` with a horizontal gradient between `from` and `to`.
pub fn gradient(text: &str, from: (u8, u8, u8), to: (u8, u8, u8)) -> String {
    if !color() {
        return text.to_owned();
    }
    let n = text.chars().count().max(2) - 1;
    let mut out = String::new();
    for (i, c) in text.chars().enumerate() {
        if c == ' ' {
            out.push(c);
        } else {
            let (r, g, b) = lerp(from, to, i as f32 / n as f32);
            out.push_str(&fg(r, g, b));
            out.push(c);
        }
    }
    out.push_str("\x1b[0m");
    out
}

/// Brand orange for accents.
pub fn accent(s: &str) -> String {
    if color() { format!("{}{s}\x1b[0m", fg(0xf2, 0x6b, 0x2a)) } else { s.to_owned() }
}

const NEXT: [&str; 6] = [
    "███╗   ██╗███████╗██╗  ██╗████████╗",
    "████╗  ██║██╔════╝╚██╗██╔╝╚══██╔══╝",
    "██╔██╗ ██║█████╗   ╚███╔╝    ██║   ",
    "██║╚██╗██║██╔══╝   ██╔██╗    ██║   ",
    "██║ ╚████║███████╗██╔╝ ██╗   ██║   ",
    "╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝   ╚═╝   ",
];

const RUST: [&str; 6] = [
    "██████╗ ██╗   ██╗███████╗████████╗",
    "██╔══██╗██║   ██║██╔════╝╚══██╔══╝",
    "██████╔╝██║   ██║███████╗   ██║   ",
    "██╔══██╗██║   ██║╚════██║   ██║   ",
    "██║  ██║╚██████╔╝███████║   ██║   ",
    "╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ",
];

/// The large NEXT / RUST wordmark with tagline.
pub fn banner() {
    eprintln!();
    for line in NEXT {
        eprintln!("   {}", gradient(line, (0xff, 0xf1, 0xe0), (0xff, 0xb3, 0x5c)));
    }
    for line in RUST {
        eprintln!("   {}", gradient(line, (0xff, 0x9a, 0x3c), (0xd9, 0x3a, 0x0e)));
    }
    eprintln!();
    eprintln!(
        "   {}  {}",
        bold(&format!("Next Rust v{}", env!("CARGO_PKG_VERSION"))),
        dim("The Rust-native full-stack web framework")
    );
    eprintln!();
}

/// One-line header for commands that do not need the full wordmark.
pub fn header(title: &str) {
    eprintln!();
    eprintln!("  {} {}  {}", accent("▲"), bold(&format!("Next Rust {}", env!("CARGO_PKG_VERSION"))), dim(title));
    eprintln!();
}

/// Draw a rounded box around lines (ANSI-aware width).
pub fn boxed(lines: &[String]) {
    let inner = lines.iter().map(|l| strip_ansi(l).chars().count()).max().unwrap_or(0) + 4;
    let border = |s: &str| dim(s);
    eprintln!("  {}", border(&format!("╭{}╮", "─".repeat(inner))));
    for l in lines {
        eprintln!("  {}  {}  {}", border("│"), pad(l, inner - 4), border("│"));
    }
    eprintln!("  {}", border(&format!("╰{}╯", "─".repeat(inner))));
}

/// A completed step with a short pause on interactive terminals, so progress
/// reads as progress instead of an instant wall of text.
pub fn done_step(label: &str, detail: &str) {
    if color() {
        std::thread::sleep(std::time::Duration::from_millis(70));
    }
    eprintln!("   {} {}  {}", green("✔"), pad(label, 22), dim(detail));
}
