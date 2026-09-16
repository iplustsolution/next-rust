//! Terminal styling (ANSI, disabled for NO_COLOR or non-terminals).

use std::io::IsTerminal;
use std::sync::OnceLock;

pub fn color() -> bool {
    static C: OnceLock<bool> = OnceLock::new();
    *C.get_or_init(|| std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal())
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
