//! Process entry point used by `next_rust::main!()`.

use next_rust_core::{Config, Environment};

use crate::app::{App, AppBuilder, Routes};

/// Load `.env` files, configure logging and run the app.
///
/// Command-line flags understood by the generated binary:
///
/// * *(none)* – serve
/// * `--export` – pre-render static pages into the build output and exit
/// * `--routes` – print the compiled route table and exit
pub fn run(routes: Routes) {
    run_with(App::new(routes))
}

/// Like [`run`], with a customized builder:
///
/// ```ignore
/// fn main() {
///     next_rust::run_with(App::new(routes()).middleware(request_id()));
/// }
/// ```
pub fn run_with(builder: AppBuilder) {
    let env = Environment::from_env();
    let config: Config = match crate::app::discover_config(builder.project_root()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    match next_rust_core::env::EnvVars::load(&config.root, env, &config.env.public_prefix) {
        Ok(vars) => vars.apply_to_process(),
        Err(e) if env.is_dev() => eprintln!("warning: could not read .env files: {e}"),
        Err(_) => {}
    }
    // Logging is configured when the app is built (see AppBuilder::build).

    let args: Vec<String> = std::env::args().skip(1).collect();
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: failed to start async runtime: {e}");
            std::process::exit(1);
        }
    };
    let app = match builder.config(config).environment(env).build() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if args.iter().any(|a| a == "--routes") {
        for (kind, pattern, source) in app.route_table() {
            println!("{kind:<5} {pattern:<40} {source}");
        }
        return;
    }
    if args.iter().any(|a| a == "--export") {
        let result = runtime.block_on(app.export());
        match result {
            Ok(report) => {
                println!("{}", serde_json::to_string(&report).unwrap_or_default());
                return;
            }
            Err(e) => {
                eprintln!("static generation failed:\n{e}");
                std::process::exit(1);
            }
        }
    }
    if let Err(e) = runtime.block_on(app.serve()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
