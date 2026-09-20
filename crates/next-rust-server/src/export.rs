//! Static generation: renders static routes into a page cache.

use std::sync::Arc;
use std::time::Instant;

use next_rust_cache::{CacheKey, CacheStore};
use next_rust_router::Params;
use serde::Serialize;

use crate::app::{AppInner, parse_pattern};
use crate::render::{StaticRender, nonce_placeholder, render_static_html};

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
    /// `dynamic` holes in the page: above zero, `bytes` is its static shell.
    pub holes: usize,
}

pub(crate) async fn prerender(inner: &Arc<AppInner>, store: &dyn CacheStore) -> Result<ExportReport, String> {
    let mut report = ExportReport::default();
    let mut failures = Vec::new();

    for (index, def) in inner.routes.pages.iter().enumerate() {
        if !def.rendering.is_prerendered() || def.intercept_from.is_some() {
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
                Ok(render) if render.status == 200 => {
                    let revalidate = render.revalidate(inner, index);
                    let entry = render.entry(inner, index, &path);
                    let bytes = render.html.replace(&format!(" nonce=\"{}\"", nonce_placeholder()), "").len();
                    let holes = render.shell.as_ref().map_or(0, |s| s.holes());
                    store.set(CacheKey::page(&path), entry).await.map_err(|e| e.to_string())?;
                    report.pages.push(ExportedPage {
                        pattern: def.pattern.to_owned(),
                        path,
                        params,
                        bytes,
                        millis: start.elapsed().as_millis(),
                        revalidate,
                        holes,
                    });
                }
                Ok(StaticRender { status, .. }) => {
                    failures.push(format!("{} ({path}): rendered with status {status}", def.source))
                }
                Err(res) => {
                    let status = res.status;
                    let text = res.into_text().await;
                    failures.push(format!("{} ({path}): {status}\n{text}", def.source));
                }
            }
        }
    }

    if failures.is_empty() { Ok(report) } else { Err(failures.join("\n\n")) }
}
