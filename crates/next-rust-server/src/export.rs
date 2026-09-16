//! Static generation (`next-rust build`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use next_rust_cache::{CacheEntry, CacheKey, CacheStore, FileStore};
use next_rust_router::Params;
use serde::Serialize;

use crate::app::{AppInner, Rendering, parse_pattern};
use crate::render::{NONCE_PLACEHOLDER, render_static_html};

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExportReport {
    pub pages: Vec<ExportedPage>,
    /// Static routes with dynamic segments but no `generate_params` (rendered on first request).
    pub on_demand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportedPage {
    pub pattern: String,
    pub path: String,
    pub params: Params,
    pub bytes: usize,
    pub millis: u128,
    pub revalidate: Option<u64>,
}

fn html_file(static_dir: &Path, path: &str) -> PathBuf {
    let mut p = static_dir.to_path_buf();
    for seg in path.split('/').filter(|s| !s.is_empty()) {
        p.push(next_rust_router::decode_segment(seg).unwrap_or_else(|| seg.to_owned()));
    }
    p.join("index.html")
}

pub(crate) async fn export(inner: &Arc<AppInner>) -> Result<ExportReport, String> {
    let out = &inner.output_dir;
    let static_dir = out.join("static");
    let store = FileStore::new(out.join("cache/pages"));
    store.clear().await.map_err(|e| e.to_string())?;
    let _ = tokio::fs::remove_dir_all(&static_dir).await;
    let mut report = ExportReport::default();
    let mut failures = Vec::new();

    for (index, def) in inner.routes.pages.iter().enumerate() {
        if def.rendering != Rendering::Static || def.intercept_from.is_some() {
            continue;
        }
        let pattern = parse_pattern(def.pattern)?;
        let list: Vec<Params> = if pattern.is_dynamic() {
            match def.generate_params {
                Some(f) => f()
                    .await
                    .map_err(|e| format!("{}: generate_params failed: {}", def.source, e.detailed_message()))?,
                None => {
                    report.on_demand.push(def.pattern.to_owned());
                    continue;
                }
            }
        } else {
            vec![Params::new()]
        };
        let mut seen = std::collections::HashSet::new();
        for params in list {
            let Some(path) = pattern.to_path(&params) else {
                failures.push(format!(
                    "{}: generate_params returned {:?}, which is missing parameters",
                    def.source, params
                ));
                continue;
            };
            if !seen.insert(path.clone()) {
                continue;
            }
            let start = Instant::now();
            match render_static_html(inner, index, &path, params.clone()).await {
                Ok((html, 200)) => {
                    let revalidate = def.revalidate.or(inner.config.rendering.revalidate);
                    let entry = CacheEntry::new(html.clone().into_bytes())
                        .revalidate(revalidate.map(Duration::from_secs))
                        .tags(def.tags.iter().copied());
                    store.set(CacheKey::page(&path), entry).await.map_err(|e| e.to_string())?;
                    let file = html_file(&static_dir, &path);
                    if let Some(parent) = file.parent() {
                        tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
                    }
                    let plain = html.replace(&format!(" nonce=\"{NONCE_PLACEHOLDER}\""), "");
                    tokio::fs::write(&file, &plain).await.map_err(|e| e.to_string())?;
                    report.pages.push(ExportedPage {
                        pattern: def.pattern.to_owned(),
                        path,
                        params,
                        bytes: plain.len(),
                        millis: start.elapsed().as_millis(),
                        revalidate,
                    });
                }
                Ok((_, status)) => failures.push(format!("{} ({path}): rendered with status {status}", def.source)),
                Err(res) => {
                    let status = res.status;
                    let text = res.into_text().await;
                    failures.push(format!("{} ({path}): {status}\n{text}", def.source));
                }
            }
        }
    }

    if let Some(f) = inner.routes.sitemap {
        write(&static_dir.join("sitemap.xml"), f().await.to_xml().as_bytes()).await?;
    }
    if let Some(f) = inner.routes.robots {
        write(&static_dir.join("robots.txt"), f().await.to_text().as_bytes()).await?;
    }

    if failures.is_empty() { Ok(report) } else { Err(failures.join("\n\n")) }
}

async fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }
    tokio::fs::write(path, bytes).await.map_err(|e| e.to_string())
}
