//! The Next Rust wordmark and brand colours for terminal output, shared by
//! the CLI and the production server's startup screen.

/// "NEXT" in block letters.
pub const NEXT: [&str; 6] = [
    "███╗   ██╗███████╗██╗  ██╗████████╗",
    "████╗  ██║██╔════╝╚██╗██╔╝╚══██╔══╝",
    "██╔██╗ ██║█████╗   ╚███╔╝    ██║   ",
    "██║╚██╗██║██╔══╝   ██╔██╗    ██║   ",
    "██║ ╚████║███████╗██╔╝ ██╗   ██║   ",
    "╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝   ╚═╝   ",
];

/// "RUST" in block letters.
pub const RUST: [&str; 6] = [
    "██████╗ ██╗   ██╗███████╗████████╗",
    "██╔══██╗██║   ██║██╔════╝╚══██╔══╝",
    "██████╔╝██║   ██║███████╗   ██║   ",
    "██╔══██╗██║   ██║╚════██║   ██║   ",
    "██║  ██║╚██████╔╝███████║   ██║   ",
    "╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ",
];

/// Brand orange.
pub const ACCENT: (u8, u8, u8) = (0xf2, 0x6b, 0x2a);

/// Whether the terminal advertises 24-bit colour.
fn truecolor() -> bool {
    std::env::var("COLORTERM").is_ok_and(|v| v.contains("truecolor") || v.contains("24bit"))
}

/// Foreground colour escape for an RGB value (24-bit, or the nearest of the
/// 256-colour palette on terminals like macOS Terminal.app).
pub fn fg((r, g, b): (u8, u8, u8)) -> String {
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
pub fn gradient(text: &str, from: (u8, u8, u8), to: (u8, u8, u8), color: bool) -> String {
    if !color {
        return text.to_owned();
    }
    let n = text.chars().count().max(2) - 1;
    let mut out = String::new();
    for (i, c) in text.chars().enumerate() {
        if c == ' ' {
            out.push(c);
        } else {
            out.push_str(&fg(lerp(from, to, i as f32 / n as f32)));
            out.push(c);
        }
    }
    out.push_str("\x1b[0m");
    out
}

/// `text` in the brand orange.
pub fn accent(text: &str, color: bool) -> String {
    if color { format!("{}{text}\x1b[0m", fg(ACCENT)) } else { text.to_owned() }
}

/// The NEXT / RUST wordmark, one string per line, indented by three spaces.
pub fn wordmark(color: bool) -> Vec<String> {
    let next = NEXT.iter().map(|l| gradient(l, (0xff, 0xf1, 0xe0), (0xff, 0xb3, 0x5c), color));
    let rust = RUST.iter().map(|l| gradient(l, (0xff, 0x9a, 0x3c), (0xd9, 0x3a, 0x0e), color));
    next.chain(rust).map(|l| format!("   {l}")).collect()
}

/// Whether to colour output written to a stream that is (or is not) a
/// terminal, honouring `NO_COLOR`, `FORCE_COLOR` and `CLICOLOR_FORCE`.
pub fn color_enabled(is_terminal: bool) -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    let forced = ["FORCE_COLOR", "CLICOLOR_FORCE"].iter().any(|k| std::env::var(k).is_ok_and(|v| v != "0"));
    forced || is_terminal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_wordmark_without_color() {
        let lines = wordmark(false);
        assert_eq!(lines.len(), 12);
        assert!(lines.iter().all(|l| !l.contains('\x1b')));
        assert_eq!(accent("x", false), "x");
    }
}
