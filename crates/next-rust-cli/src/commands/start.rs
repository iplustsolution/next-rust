use std::process::Command;

use crate::{Args, project};

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!("next-rust start [--port <port>]\n\nRun the production server built by `next-rust build`.");
        return Ok(());
    }
    let info = project::load()?;
    let exe = info.config.output_dir().join(format!("{}{}", info.bin_name, std::env::consts::EXE_SUFFIX));
    if !exe.is_file() {
        return Err(format!("no production build found at {}; run `next-rust build` first", exe.display()));
    }
    let mut cmd = Command::new(&exe);
    cmd.env("NEXT_RUST_ENV", "production");
    if let Some(port) = a.value(&["--port", "-p"]) {
        cmd.env("PORT", port);
    }
    let status = cmd.status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(String::new()) }
}
