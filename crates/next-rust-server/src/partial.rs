//! Partial prerendering: a static shell with per-request holes, and cached
//! fragments.
//!
//! ```ignore
//! pub const RENDERING: Rendering = Rendering::Partial;
//! pub const REVALIDATE: u64 = 3600;
//!
//! pub async fn Page() -> Result<impl View> {
//!     let product = load_product().await?;               // runs once, for the shell
//!     Ok(main![
//!         h1![product.name],
//!         dynamic(p!["Loading your cart…"], |ctx| async move {
//!             let cart = cart_for(ctx.cookies.get("session")).await?;   // runs per request
//!             Ok::<_, Error>(p![format!("{} items", cart.len())])
//!         }),
//!     ])
//! }
//! ```
//!
//! The shell — everything outside the holes, with the fallbacks in place — is
//! rendered with a static context (request extractors fail there, so it cannot
//! hold per-user data) and cached under the page's `REVALIDATE`/`TAGS` like
//! any static page. A request is answered with the shell bytes at once; the
//! holes then run concurrently with the real request context and are streamed
//! into the same response (`<template id="nr-tN">` + `$nr(N)`, see
//! `next_rust_view::render`).
//!
//! The shell's HTML lives in the page store. The holes are closures, so they
//! live in memory next to it ([`Shells`]), tied to the HTML by a token. For a
//! cached shell whose holes this process does not have (a restart with a
//! `FileStore`, an entry written by another instance), the page is rendered
//! here once and kept in memory for as long as that entry lives.

use std::collections::HashMap;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::FutureExt;
use next_rust_cache::{CacheEntry, CacheKey, CacheOptions};
use next_rust_view::{IntoViewResult, Node, Rendered, Shell, Stylesheet, View, hole, render_fragment};
use serde::{Deserialize, Serialize};

use crate::app::ErrorInfo;
use crate::context::Ctx;
use crate::error::{Error, Result};
use crate::middleware::BoxFuture;

/// Meta key of a page-store entry that is a shell: its value is the token of
/// the matching [`Shells`] entry.
pub(crate) const SHELL_META: &str = "nr-shell";

/// Holes of the cached shells, by cache path. The shell being replaced by a
/// revalidation is kept next to the new one: requests may still read its HTML.
///
/// A page with `dynamic_params` and no `generate_params` has as many paths as
/// a client cares to ask for, so the map is bounded. It gives up its
/// least-recently-used paths rather than everything it holds: a visitor
/// asking for many one-off URLs would otherwise make every live page render
/// its shell again, over and over.
#[derive(Default)]
pub(crate) struct Shells(Mutex<ShellMap>);

#[derive(Default)]
struct ShellMap {
    paths: HashMap<String, PathShells>,
    clock: u64,
}

struct PathShells {
    /// `clock` when this path was last read or written.
    used: u64,
    shells: Vec<LocalShell>,
}

impl ShellMap {
    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// Drop the `count` least recently used paths. Done in batches, so the
    /// scan is amortized over that many inserts.
    fn evict(&mut self, count: usize) {
        let mut used: Vec<u64> = self.paths.values().map(|e| e.used).collect();
        if used.len() <= count {
            self.paths.clear();
            return;
        }
        used.sort_unstable();
        let cutoff = used[count];
        self.paths.retain(|_, e| e.used >= cutoff);
    }
}

/// The holes that belong to the page-store entry carrying `token`.
#[derive(Clone)]
pub(crate) struct LocalShell {
    token: String,
    pub(crate) shell: Arc<Shell>,
    /// Set when the entry was written by someone else (another instance
    /// sharing the store, or this app before a restart): the HTML rendered
    /// here, which is the one these holes fit. The entry itself is left
    /// alone, so instances do not overwrite each other's shells in turn.
    pub(crate) html: Option<Arc<str>>,
}

impl Shells {
    /// More paths than this (pages with many parameter values): the oldest go.
    const MAX: usize = 10_000;
    /// How many to drop at once when the map is full.
    const EVICT: usize = Self::MAX / 10;

    pub(crate) fn get(&self, path: &str, token: &str) -> Option<LocalShell> {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = map.tick();
        let entry = map.paths.get_mut(path)?;
        entry.used = now;
        entry.shells.iter().find(|s| s.token == token).cloned()
    }

    pub(crate) fn insert(&self, path: &str, token: &str, shell: Arc<Shell>, html: Option<Arc<str>>) {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = map.tick();
        if map.paths.len() >= Self::MAX && !map.paths.contains_key(path) {
            map.evict(Self::EVICT);
        }
        let entry = map.paths.entry(path.to_owned()).or_insert(PathShells { used: now, shells: Vec::new() });
        entry.used = now;
        entry.shells.retain(|s| s.token != token);
        entry.shells.push(LocalShell { token: token.to_owned(), shell, html });
        if entry.shells.len() > 2 {
            entry.shells.remove(0);
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).paths.len()
    }
}

/// What the server renders holes with.
pub(crate) struct HoleCx {
    pub(crate) ctx: Ctx,
    /// Renders an error of a hole: the nearest `error.rs`/`not-found.rs` of
    /// the page, a redirect script, or the default message.
    pub(crate) on_error: Box<dyn Fn(Error) -> BoxFuture<Node> + Send + Sync>,
}

type ErrorView = Arc<dyn Fn(ErrorInfo) -> Node + Send + Sync>;

/// A dynamic hole with options: `Dynamic::new(fallback).on_error(view).render(f)`.
/// [`dynamic`] is the short form.
pub struct Dynamic {
    fallback: Node,
    error: Option<ErrorView>,
}

impl Dynamic {
    /// `fallback` is part of the static shell: what is shown until the hole
    /// arrives (and what stays when JavaScript is off).
    pub fn new(fallback: impl View) -> Self {
        Dynamic { fallback: fallback.into_node(), error: None }
    }

    /// Render errors of this hole here instead of in the page's `error.rs`.
    pub fn on_error<V: View>(mut self, view: impl Fn(ErrorInfo) -> V + Send + Sync + 'static) -> Self {
        self.error = Some(Arc::new(move |info| view(info).into_node()));
        self
    }

    /// The content: called for every request with its context. Whatever it
    /// captures comes from the static render and is shared by all requests.
    pub fn render<F, Fut, R>(self, f: F) -> Node
    where
        F: Fn(Ctx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoViewResult<Error> + 'static,
    {
        let f = Arc::new(f);
        let error = self.error;
        hole(self.fallback, move |cx| {
            let (f, error) = (f.clone(), error.clone());
            Box::pin(async move {
                // Rendered by something other than the server: nothing to run with.
                let Ok(cx) = cx.downcast::<HoleCx>() else { return Node::Empty };
                let before = Effects::of(&cx.ctx);
                let result =
                    AssertUnwindSafe(async { f(cx.ctx.clone()).await.into_view_result() }).catch_unwind().await;
                before.warn_if_changed(&cx.ctx);
                let e = match result {
                    Ok(Ok(node)) => return node,
                    Ok(Err(e)) => e,
                    Err(_) => Error::msg("a dynamic hole panicked"),
                };
                match &error {
                    Some(view) if !e.is_redirect() => view(ErrorInfo::from_error(&e, cx.ctx.dev)),
                    _ => (cx.on_error)(e).await,
                }
            })
        })
    }
}

/// A per-request hole in an otherwise static page or layout.
///
/// `fallback` belongs to the static shell. `f` runs for every request with the
/// request's context (`ctx.cookies`, `ctx.headers`, `ctx.uri`,
/// `ctx.extract::<Query>()?`, …) and returns a view or a `Result` of one; its
/// output is streamed into the response after the shell. On a dynamic page a
/// hole behaves like `suspense`.
///
/// The response head is sent before holes run: a hole cannot set cookies,
/// response headers or the status (a warning is logged when one tries).
pub fn dynamic<F, Fut, R>(fallback: impl View, f: F) -> Node
where
    F: Fn(Ctx) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoViewResult<Error> + 'static,
{
    Dynamic::new(fallback).render(f)
}

/// What a render may change on the response. Holes run after the response
/// head is sent, so changes they make are lost.
struct Effects {
    cookies: Vec<String>,
    headers: usize,
    status: u16,
}

impl Effects {
    fn of(ctx: &Ctx) -> Self {
        Effects {
            cookies: ctx.cookies.set_cookie_headers(),
            headers: ctx.response_headers.lock().unwrap_or_else(|e| e.into_inner()).len(),
            status: ctx.status(),
        }
    }

    fn warn_if_changed(&self, ctx: &Ctx) {
        let now = Effects::of(ctx);
        let what = if now.cookies != self.cookies {
            "set a cookie"
        } else if now.headers != self.headers {
            "set a response header"
        } else if now.status != self.status {
            "set the response status"
        } else {
            return;
        };
        crate::log::warn(&format!(
            "a dynamic hole on {} {what} after the response head was sent: it has no effect. \
             Do it in middleware, a server action or a route handler instead.",
            ctx.path()
        ));
    }
}

// ---------------------------------------------------------------------------
// Cached fragments
// ---------------------------------------------------------------------------

/// Stylesheets are statics: a cached fragment names them by id.
static SHEETS: Mutex<Option<HashMap<&'static str, &'static Stylesheet>>> = Mutex::new(None);

#[derive(Serialize, Deserialize)]
struct StoredFragment {
    html: String,
    sheets: Vec<String>,
    classes: Vec<String>,
    scripts: Vec<String>,
    /// links, islands, streamed, ui
    flags: [bool; 4],
    /// A `<form>` is inside (entries stored before this existed: none known).
    #[serde(default)]
    forms: bool,
}

impl StoredFragment {
    fn from_rendered(r: &Rendered) -> Self {
        let mut known = SHEETS.lock().unwrap_or_else(|e| e.into_inner());
        let known = known.get_or_insert_with(HashMap::new);
        for sheet in &r.sheets {
            known.insert(sheet.id, sheet);
        }
        StoredFragment {
            html: r.html.clone(),
            sheets: r.sheets.iter().map(|s| s.id.to_owned()).collect(),
            classes: r.classes.clone(),
            scripts: r.scripts.clone(),
            flags: [r.flags.links, r.flags.islands, r.flags.streamed, r.flags.ui],
            forms: r.flags.forms,
        }
    }

    /// `None` when a stylesheet is unknown to this process (an entry from
    /// another build in a persistent store): render again.
    fn into_rendered(self) -> Option<Rendered> {
        let known = SHEETS.lock().unwrap_or_else(|e| e.into_inner());
        let sheets = self
            .sheets
            .iter()
            .map(|id| known.as_ref().and_then(|k| k.get(id.as_str()).copied()))
            .collect::<Option<Vec<_>>>()?;
        let [links, islands, streamed, ui] = self.flags;
        Some(Rendered {
            html: self.html,
            sheets,
            classes: self.classes,
            scripts: self.scripts,
            flags: next_rust_view::RenderFlags { links, islands, streamed, ui, forms: self.forms },
        })
    }
}

/// Lifetime and tags of the fragments used by a render.
#[derive(Debug, Default, Clone)]
pub(crate) struct FragmentUse {
    pub(crate) tags: Vec<String>,
    pub(crate) revalidate: Option<Duration>,
}

tokio::task_local! {
    static FRAGMENTS: Mutex<FragmentUse>;
}

/// Run a static render and report the cached fragments it used: the page
/// that contains them is cached under their tags too, and no longer than the
/// shortest-lived of them.
pub(crate) async fn track_fragments<T>(render: impl Future<Output = T>) -> (T, FragmentUse) {
    FRAGMENTS
        .scope(Mutex::new(FragmentUse::default()), async {
            let out = render.await;
            let used = FRAGMENTS.with(|f| f.lock().unwrap_or_else(|e| e.into_inner()).clone());
            (out, used)
        })
        .await
}

fn note_fragment(opts: &CacheOptions) {
    let _ = FRAGMENTS.try_with(|f| {
        let mut used = f.lock().unwrap_or_else(|e| e.into_inner());
        for tag in &opts.tags {
            if !used.tags.contains(tag) {
                used.tags.push(tag.clone());
            }
        }
        used.revalidate = match (used.revalidate, opts.revalidate) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    });
}

fn fragment_key(key: &str) -> CacheKey {
    CacheKey::new(format!("fragment:{key}"))
}

/// Render a part of a page once and reuse its HTML: fragment caching.
///
/// ```ignore
/// let sidebar = cached("sidebar", CacheOptions::revalidate(300).tag("posts"), || async {
///     let posts = recent_posts().await?;
///     Ok::<_, Error>(ul![each(posts, |p| li![p.title])])
/// })
/// .await?;
/// ```
///
/// The entry lives in the data cache under `key` with the options' lifetime
/// and tags, so `revalidate_tag("posts")` (or [`invalidate_fragment`]) renders
/// it again on its own. Inside a hole that happens on the next request. A
/// static page or shell that contains the fragment is cached under the
/// fragment's tags too and for no longer than the fragment lives, so it is
/// rendered again with it — and only the fragments that are stale are
/// computed again. A stale entry is rendered again before it is used; errors
/// are never cached.
///
/// The key must say everything the view depends on (`format!("price:{id}")`).
/// Never cache per-user markup under a shared key. Suspense inside the view is
/// awaited; a `dynamic` hole inside it shows its fallback — put the hole
/// around the cached fragment, not in it.
pub async fn cached<F, Fut, R>(key: &str, opts: CacheOptions, f: F) -> Result<Node>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = R>,
    R: IntoViewResult<Error>,
{
    note_fragment(&opts);
    let store = next_rust_cache::global().store().clone();
    let key = fragment_key(key);
    if let Ok(Some(entry)) = store.get(&key).await
        && !entry.is_stale()
        && let Ok(stored) = serde_json::from_slice::<StoredFragment>(&entry.value)
        && let Some(rendered) = stored.into_rendered()
    {
        return Ok(Node::Rendered(Box::new(rendered)));
    }
    let rendered = render_fragment(f().await.into_view_result()?).await;
    if let Ok(bytes) = serde_json::to_vec(&StoredFragment::from_rendered(&rendered)) {
        let entry = CacheEntry::new(bytes).revalidate(opts.revalidate).tags(opts.tags);
        if let Err(e) = store.set(key, entry).await {
            crate::log::warn(&format!("could not cache a fragment: {e}"));
        }
    }
    Ok(Node::Rendered(Box::new(rendered)))
}

/// Drop one cached fragment; it is rendered again where it is next used.
pub async fn invalidate_fragment(key: &str) {
    let _ = next_rust_cache::global().store().delete(&fragment_key(key)).await;
}

#[cfg(test)]
mod tests {
    use next_rust_view::DocumentParts;

    use super::*;

    async fn a_shell() -> Arc<Shell> {
        let body = hole("fallback", |_cx| Box::pin(async { Node::Empty }));
        let (_, shell) = next_rust_view::render_shell(DocumentParts::new(body)).await;
        Arc::new(shell.expect("a hole makes a shell"))
    }

    /// A page whose parameters a client chooses (`dynamic_params` without
    /// `generate_params`) can fill the map with one-off paths. It used to be
    /// cleared completely at that point, so every page still in use rendered
    /// its shell again — repeatedly, for as long as the requests kept coming.
    #[tokio::test]
    async fn a_full_shell_map_gives_up_its_oldest_paths_not_all_of_them() {
        let shells = Shells::default();
        let shell = a_shell().await;
        for i in 0..Shells::MAX - 1 {
            shells.insert(&format!("/junk/{i}"), "t", shell.clone(), None);
        }
        // The page real visitors are on, asked for last and so the newest.
        shells.insert("/hot", "t", shell.clone(), None);
        assert_eq!(shells.len(), Shells::MAX);
        assert!(shells.get("/hot", "t").is_some());

        // One path more than fits.
        shells.insert("/junk/overflow", "t", shell.clone(), None);
        assert_eq!(shells.len(), Shells::MAX - Shells::EVICT + 1, "a batch of the oldest went, not everything");
        assert!(shells.get("/hot", "t").is_some(), "the page in use is kept");
        assert!(shells.get("/junk/overflow", "t").is_some());
        assert!(shells.get("/junk/0", "t").is_none(), "the oldest path went");

        // Reading a path keeps it: a hot page survives any number of evictions.
        for i in 0..Shells::MAX {
            assert!(shells.get("/hot", "t").is_some(), "evicted after {i} more paths");
            shells.insert(&format!("/more/{i}"), "t", shell.clone(), None);
        }
        assert!(shells.len() <= Shells::MAX);
    }

    /// A token names one entry of one path: nothing else can be served for it.
    #[tokio::test]
    async fn a_shell_is_only_served_for_its_own_path_and_token() {
        let shells = Shells::default();
        let shell = a_shell().await;
        shells.insert("/a", "token-a", shell.clone(), Some("<p>a</p>".into()));
        shells.insert("/b", "token-b", shell.clone(), Some("<p>b</p>".into()));
        assert!(shells.get("/a", "token-b").is_none());
        assert!(shells.get("/b", "token-a").is_none());
        assert!(shells.get("/a", "").is_none());
        assert_eq!(shells.get("/a", "token-a").unwrap().html.as_deref(), Some("<p>a</p>"));

        // Two shells per path at most: a third drops the oldest.
        for token in ["t1", "t2", "t3"] {
            shells.insert("/a", token, shell.clone(), None);
        }
        assert!(shells.get("/a", "token-a").is_none() && shells.get("/a", "t1").is_none());
        assert!(shells.get("/a", "t2").is_some() && shells.get("/a", "t3").is_some());
    }
}
