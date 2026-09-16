use crate::{Args, project, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    if Args::new(args).flag(&["-h", "--help"]) {
        println!(
            "next-rust analyze\n\nSummarize the last production build: rendering modes, pre-rendered sizes, largest pages and binary size."
        );
        return Ok(());
    }
    let config = project::load_config()?;
    let out = config.output_dir();
    let read = |p: &str| {
        std::fs::read_to_string(out.join(p))
            .map_err(|_| format!("{} not found; run `next-rust build` first", out.join(p).display()))
    };
    let manifest: serde_json::Value =
        serde_json::from_str(&read("manifest/routes.json")?).map_err(|e| e.to_string())?;
    let build: serde_json::Value = serde_json::from_str(&read("manifest/build.json")?).unwrap_or_default();

    let routes = manifest["routes"].as_array().cloned().unwrap_or_default();
    let count = |k: &str, v: &str| routes.iter().filter(|r| r[k] == v).count();
    println!("{}", ui::bold("Routes"));
    println!("  static   {}", count("rendering", "static"));
    println!("  dynamic  {}", routes.iter().filter(|r| r["rendering"] == "dynamic" && r["kind"] != "api").count());
    println!("  api      {}", count("kind", "api"));

    let mut pages: Vec<(String, u64, u64)> = build["pages"]
        .as_array()
        .map(|p| {
            p.iter()
                .map(|x| {
                    (
                        x["path"].as_str().unwrap_or("").to_owned(),
                        x["bytes"].as_u64().unwrap_or(0),
                        x["millis"].as_u64().unwrap_or(0),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    pages.sort_by_key(|p| std::cmp::Reverse(p.1));
    let total: u64 = pages.iter().map(|p| p.1).sum();
    println!("\n{} ({} pages, {})", ui::bold("Pre-rendered HTML"), pages.len(), ui::bytes(total));
    for (path, bytes, ms) in pages.iter().take(10) {
        println!("  {:<40} {:>10} {}", path, ui::bytes(*bytes), ui::dim(&format!("{ms} ms")));
    }
    if let Some(on_demand) = build["on_demand"].as_array() {
        for p in on_demand {
            println!("  {:<40} {}", p.as_str().unwrap_or(""), ui::dim("on demand"));
        }
    }
    if let Ok(rd) = std::fs::read_dir(out.join("server")) {
        println!("\n{}", ui::bold("Server"));
        for e in rd.flatten() {
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            println!("  {:<40} {:>10}", e.file_name().to_string_lossy(), ui::bytes(size));
        }
    }
    Ok(())
}
