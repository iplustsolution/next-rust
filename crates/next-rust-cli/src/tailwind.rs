//! Getting the Tailwind CSS engine ready before cargo runs, with visible
//! progress (the build script would otherwise download it silently).

use next_rust_build::tailwind;

use crate::project::ProjectInfo;
use crate::ui;

/// Make sure the Tailwind executable is available when the project uses
/// Tailwind. Returns whether Tailwind is enabled.
pub fn prepare(info: &ProjectInfo) -> Result<bool, String> {
    if !info.config.tailwind.enabled {
        return Ok(false);
    }
    if tailwind::installed().is_some() {
        return Ok(true);
    }
    let (_, size, _) = tailwind::asset()?;
    if !ui::interactive() {
        eprintln!("   Downloading Tailwind CSS v{} ({}, once per machine)…", tailwind::VERSION, ui::bytes(size));
    }
    let mut live = ui::LiveLine::new();
    let result = tailwind::ensure(&mut |got, total| {
        let fraction = if total == 0 { 0.0 } else { got as f64 / total as f64 };
        live.draw(
            "Tailwind CSS",
            &format!(
                "{} {}  {}  {}",
                ui::progress_bar(fraction, 24),
                ui::bold(&format!("{:>3}%", (fraction * 100.0).floor() as u32)),
                ui::dim(&format!("{} of {}", ui::bytes(got), ui::bytes(total))),
                ui::dim(&format!("downloading v{}, once per machine", tailwind::VERSION)),
            ),
        );
    });
    match result {
        Ok(_) => {
            live.finish("Tailwind CSS", &format!("v{} downloaded and verified", tailwind::VERSION));
            Ok(true)
        }
        Err(e) => {
            live.fail("Tailwind CSS", &ui::red("download failed"));
            Err(e)
        }
    }
}
