//! Minify the component stylesheet (`src/style.rs`) at compile time.

include!("src/style.rs");

fn main() {
    println!("cargo:rerun-if-changed=src/style.rs");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out.join("ui.min.css"), next_rust_assets::css::minify(UI_CSS_SOURCE)).expect("write ui.min.css");
}
