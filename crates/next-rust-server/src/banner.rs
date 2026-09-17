//! The startup screen of a production server run from a terminal: the Next
//! Rust wordmark, the addresses it listens on and how to stop it.
//!
//! It is shown only when stdout is an interactive terminal, so servers under
//! Docker, systemd or a process manager keep the production rule of printing
//! nothing but errors. `NEXT_RUST_BANNER=0` turns it off.

use std::io::{IsTerminal, Write};
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::Instant;

use next_rust_core::{Environment, brand};

static PROCESS_START: OnceLock<Instant> = OnceLock::new();

/// Remember when the process started (for "started in … ms").
pub(crate) fn mark_process_start() {
    let _ = PROCESS_START.set(Instant::now());
}

pub(crate) fn enabled(env: Environment) -> bool {
    env == Environment::Production
        && std::io::stdout().is_terminal()
        && std::env::var("NEXT_RUST_BANNER").map_or(true, |v| v != "0")
}

fn color() -> bool {
    brand::color_enabled(std::io::stdout().is_terminal())
}

fn paint(code: &str, s: &str) -> String {
    if color() { format!("\x1b[{code}m{s}\x1b[0m") } else { s.to_owned() }
}

/// The machine's address on the local network, found by asking the OS which
/// interface would route to a public address (no packet is sent).
fn lan_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

pub(crate) fn started(local: SocketAddr, pages: usize, apis: usize) {
    let c = color();
    let bold = |s: &str| paint("1", s);
    let dim = |s: &str| paint("2", s);
    let url = |host: &str| brand::accent(&format!("http://{host}:{}", local.port()), c);
    let row = |label: &str, value: String| format!("     {}{value}", dim(&format!("{label:<11}")));

    let mut out = String::from("\n");
    for line in brand::wordmark(c) {
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&format!(
        "   {}  {}\n\n",
        bold(&format!("Next Rust v{}", env!("CARGO_PKG_VERSION"))),
        dim("production server")
    ));
    let elapsed = PROCESS_START.get().map(|t| format!(" in {} ms", t.elapsed().as_millis())).unwrap_or_default();
    out.push_str(&format!("   {} {}\n\n", paint("32", "✔"), bold(&format!("Server started{elapsed}"))));

    let ip = local.ip();
    if ip.is_unspecified() {
        out.push_str(&row("Local", url("localhost")));
        out.push('\n');
        if let Some(lan) = lan_ip() {
            let host = if lan.is_ipv6() { format!("[{lan}]") } else { lan.to_string() };
            out.push_str(&row("Network", url(&host)));
            out.push('\n');
        }
    } else {
        let host = if ip.is_loopback() {
            "localhost".to_owned()
        } else if ip.is_ipv6() {
            format!("[{ip}]")
        } else {
            ip.to_string()
        };
        out.push_str(&row("Local", url(&host)));
        out.push('\n');
    }
    let routes = match apis {
        0 => plural(pages, "page"),
        n => format!("{} · {}", plural(pages, "page"), plural(n, "API route")),
    };
    out.push_str(&row("Routes", routes));
    out.push('\n');
    out.push_str(&row("Mode", "production".into()));
    out.push_str("\n\n");
    out.push_str(&format!("   {}\n\n", dim("Press Ctrl+C to stop")));

    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}

pub(crate) fn stopping() {
    println!("\n   {} {}", brand::accent("◆", color()), paint("2", "Stopping, finishing open requests…"));
}

pub(crate) fn stopped() {
    println!("   {} {}\n", paint("32", "✔"), paint("1", "Server stopped"));
}
