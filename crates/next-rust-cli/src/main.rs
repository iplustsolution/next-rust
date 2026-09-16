//! `next-rust` — the Next Rust command-line interface.

mod commands;
mod project;
mod templates;
mod ui;

use std::process::ExitCode;

const HELP: &str = "\
Next Rust — a Rust-native full-stack web framework

USAGE:
    next-rust <COMMAND> [OPTIONS]

COMMANDS:
    new <name>          Create a new project
    dev                 Start the development server (watch, rebuild, reload)
    build               Build for production and pre-render static pages
    start               Run the production build
    check               Validate routes and type-check the project
    routes              Print the route table (--json, --tree, --layouts)
    analyze             Report build output sizes and rendering modes
    generate <kind> <route>
                        Scaffold a file: page, layout, loading, error,
                        not-found, template, api, middleware
    doctor              Diagnose the project and toolchain
    docker              Write a production Dockerfile
    clean               Remove build output

OPTIONS:
    -h, --help          Print help
    -V, --version       Print version

Run `next-rust <COMMAND> --help` for command options.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    };
    let rest = &args[1..];
    let result = match command.as_str() {
        "-h" | "--help" | "help" => {
            println!("{HELP}");
            Ok(())
        }
        "-V" | "--version" | "version" => {
            println!("next-rust {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "new" => commands::new::run(rest),
        "dev" => commands::dev::run(rest),
        "build" => commands::build::run(rest),
        "start" => commands::start::run(rest),
        "check" => commands::check::run(rest),
        "routes" => commands::routes::run(rest),
        "analyze" => commands::analyze::run(rest),
        "generate" | "g" => commands::generate::run(rest),
        "doctor" => commands::doctor::run(rest),
        "docker" => commands::docker::run(rest),
        "clean" => commands::clean::run(rest),
        other => Err(format!("unknown command `{other}`\n\n{HELP}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if !e.is_empty() {
                ui::fail(&e);
            }
            ExitCode::FAILURE
        }
    }
}

/// Minimal flag parsing helpers.
pub struct Args<'a> {
    args: &'a [String],
}

impl<'a> Args<'a> {
    pub fn new(args: &'a [String]) -> Self {
        Args { args }
    }

    pub fn flag(&self, names: &[&str]) -> bool {
        self.args.iter().any(|a| names.contains(&a.as_str()))
    }

    pub fn value(&self, names: &[&str]) -> Option<&'a str> {
        self.args.iter().enumerate().find_map(|(i, a)| {
            if names.contains(&a.as_str()) {
                self.args.get(i + 1).map(String::as_str)
            } else {
                names.iter().find_map(|n| a.strip_prefix(&format!("{n}=")))
            }
        })
    }

    /// Positional arguments (not flags or flag values).
    pub fn positional(&self, flags_with_values: &[&str]) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut skip = false;
        for a in self.args {
            if skip {
                skip = false;
                continue;
            }
            if a.starts_with('-') {
                skip = flags_with_values.contains(&a.as_str());
                continue;
            }
            out.push(a.as_str());
        }
        out
    }
}
