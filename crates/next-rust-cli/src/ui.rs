//! Terminal styling (ANSI, disabled for NO_COLOR or non-terminals).

use std::io::IsTerminal;
use std::sync::OnceLock;

use next_rust_core::brand;

pub fn color() -> bool {
    static C: OnceLock<bool> = OnceLock::new();
    *C.get_or_init(|| brand::color_enabled(std::io::stderr().is_terminal()))
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

/// Brand orange for accents.
pub fn accent(s: &str) -> String {
    brand::accent(s, color())
}

/// The large NEXT / RUST wordmark with tagline.
pub fn banner() {
    eprintln!();
    for line in brand::wordmark(color()) {
        eprintln!("{line}");
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

// ---------------------------------------------------------------------------
// Live progress
// ---------------------------------------------------------------------------

/// Whether progress can be redrawn in place: stderr is an interactive
/// terminal (not CI logs or a pipe).
pub fn interactive() -> bool {
    std::io::stderr().is_terminal() && std::env::var_os("CI").is_none()
}

/// Terminal width in columns (`COLUMNS`, then `stty size`, else 100).
pub fn term_width() -> usize {
    static W: OnceLock<usize> = OnceLock::new();
    *W.get_or_init(|| {
        if let Some(cols) = std::env::var("COLUMNS").ok().and_then(|c| c.parse().ok()) {
            return cols;
        }
        std::fs::File::open("/dev/tty")
            .ok()
            .and_then(|tty| std::process::Command::new("stty").arg("size").stdin(tty).output().ok())
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .and_then(|s| s.split_whitespace().nth(1).and_then(|c| c.parse().ok()))
            .unwrap_or(100)
    })
}

/// Cut `s` to `width` visible characters (ANSI-aware), ending with `…`.
pub fn truncate(s: &str, width: usize) -> String {
    if strip_ansi(s).chars().count() <= width {
        return s.to_owned();
    }
    let mut out = String::new();
    let (mut visible, mut in_escape) = (0, false);
    for c in s.chars() {
        match (in_escape, c) {
            (false, '\x1b') => {
                in_escape = true;
                out.push(c);
            }
            (true, 'm') => {
                in_escape = false;
                out.push(c);
            }
            (true, _) => out.push(c),
            (false, _) if visible + 1 < width => {
                visible += 1;
                out.push(c);
            }
            (false, _) => break,
        }
    }
    out.push('…');
    if color() {
        out.push_str("\x1b[0m");
    }
    out
}

/// `███████░░░░░` for a fraction between 0 and 1.
pub fn progress_bar(fraction: f64, width: usize) -> String {
    let filled = ((fraction.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    format!("{}{}", accent(&"█".repeat(filled)), dim(&"░".repeat(width - filled)))
}

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// A status line redrawn in place while a step runs, then replaced by a
/// finished `✔` line. Outside interactive terminals nothing is redrawn.
pub struct LiveLine {
    frame: usize,
    drawn: bool,
}

impl LiveLine {
    pub fn new() -> Self {
        LiveLine { frame: 0, drawn: false }
    }

    /// Redraw `label  detail` with the next spinner frame.
    pub fn draw(&mut self, label: &str, detail: &str) {
        if !interactive() {
            return;
        }
        use std::io::Write;
        self.frame = (self.frame + 1) % SPINNER.len();
        let line = format!("   {} {}  {}", accent(SPINNER[self.frame]), pad(&bold(label), 22), detail);
        let mut err = std::io::stderr().lock();
        let _ = write!(err, "\r\x1b[2K{}", truncate(&line, term_width().saturating_sub(1)));
        let _ = err.flush();
        self.drawn = true;
    }

    /// Remove the live line so normal output can follow.
    pub fn clear(&mut self) {
        if self.drawn {
            use std::io::Write;
            let mut err = std::io::stderr().lock();
            let _ = write!(err, "\r\x1b[2K");
            let _ = err.flush();
            self.drawn = false;
        }
    }

    /// Replace the live line with a finished step.
    pub fn finish(&mut self, label: &str, detail: &str) {
        self.clear();
        eprintln!("   {} {}  {}", green("✔"), pad(label, 22), dim(detail));
    }

    /// Replace the live line with a failed step.
    pub fn fail(&mut self, label: &str, detail: &str) {
        self.clear();
        eprintln!("   {} {}  {}", red("✗"), pad(&bold(label), 22), detail);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_by_visible_width() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(strip_ansi(&truncate("hello world", 6)), "hello…");
        assert_eq!(strip_ansi(&progress_bar(0.5, 10)), "█████░░░░░");
        assert_eq!(strip_ansi(&progress_bar(2.0, 4)), "████");
    }
}
