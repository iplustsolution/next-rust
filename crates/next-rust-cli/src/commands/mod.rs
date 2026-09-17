pub mod analyze;
pub mod build;
pub mod check;
pub mod clean;
pub mod dev;
pub mod docker;
pub mod doctor;
pub mod editor;
pub mod generate;
pub mod new;
pub mod routes;
pub mod start;
pub mod upgrade;

use next_rust_build::Project;

use crate::ui;

/// Print diagnostics; returns `Err` if there are errors.
pub fn report(project: &Project) -> Result<(), String> {
    for d in project.diagnostics.iter() {
        eprintln!("{}", d.render(ui::color()));
    }
    let errors = project.diagnostics.errors().count();
    if errors > 0 { Err(format!("{errors} error{} found", if errors == 1 { "" } else { "s" })) } else { Ok(()) }
}
