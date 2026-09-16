//! `next-rust.toml` configuration.
//!
//! Every section and key is optional; an empty file (or no file at all) is a
//! valid, fully working configuration.
//!
//! ```toml
//! [app]
//! directory = "app"          # any relative or absolute path
//! public = "public"
//!
//! [server]
//! host = "0.0.0.0"
//! port = 3000
//!
//! [build]
//! output = ".next-rust"
//!
//! [rendering]
//! default = "auto"
//! ```
//!
//! ## Discovery
//!
//! 1. If `NEXT_RUST_CONFIG` is set, that file is used.
//! 2. Otherwise, starting from the working directory and walking up through
//!    its ancestors, the first directory containing `next-rust.toml` or
//!    `next-rust.json` wins. Having both in one directory is an error.
//! 3. If nothing is found, defaults are used with the working directory as
//!    the project root.
//!
//! ## Path resolution
//!
//! Relative paths are resolved against the directory containing the
//! configuration file (the *project root*), never against the process working
//! directory. `..` components are allowed, which makes monorepo layouts like
//! `directory = "../shared/pages"` possible. Paths are normalized lexically.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::mode::RenderingMode;

pub const TOML_FILE: &str = "next-rust.toml";
pub const JSON_FILE: &str = "next-rust.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Project root (directory containing the config file). Not read from the file.
    #[serde(skip)]
    pub root: PathBuf,
    /// The config file that was loaded, if any.
    #[serde(skip)]
    pub source: Option<PathBuf>,

    pub app: AppConfig,
    pub api: ApiConfig,
    pub server: ServerConfig,
    pub build: BuildConfig,
    pub assets: AssetsConfig,
    pub rendering: RenderingConfig,
    pub images: ImagesConfig,
    pub security: SecurityConfig,
    pub env: EnvConfig,
    pub dev: DevConfig,
    pub logging: LoggingConfig,
    pub redirects: Vec<RedirectRule>,
    pub headers: Vec<HeaderRule>,
    /// Free-form plugin configuration: `[plugins.my-plugin]`.
    pub plugins: BTreeMap<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            source: None,
            app: AppConfig::default(),
            api: ApiConfig::default(),
            server: ServerConfig::default(),
            build: BuildConfig::default(),
            assets: AssetsConfig::default(),
            rendering: RenderingConfig::default(),
            images: ImagesConfig::default(),
            security: SecurityConfig::default(),
            env: EnvConfig::default(),
            dev: DevConfig::default(),
            logging: LoggingConfig::default(),
            redirects: Vec::new(),
            headers: Vec::new(),
            plugins: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    /// Routing directory, scanned recursively.
    pub directory: PathBuf,
    /// Static files served from the site root.
    pub public: PathBuf,
    /// Mount the whole application under a sub-path, e.g. `/docs`.
    pub base_path: String,
    /// `false`: `/about/` redirects to `/about`. `true`: the reverse.
    pub trailing_slash: bool,
    /// `lang` attribute of the `<html>` element.
    pub lang: String,
    /// What to do when a directory contains both `page.rs` and `page.html`.
    pub html_precedence: HtmlPrecedence,
    /// Follow symbolic links while scanning (loops are detected).
    pub follow_symlinks: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("app"),
            public: PathBuf::from("public"),
            base_path: String::new(),
            trailing_slash: false,
            lang: "en".into(),
            html_precedence: HtmlPrecedence::Error,
            follow_symlinks: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HtmlPrecedence {
    /// Report a build error (default).
    #[default]
    Error,
    /// `page.rs` wins; `page.html` is ignored with a warning.
    Rs,
    /// `page.html` wins; `page.rs` is ignored with a warning.
    Html,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiConfig {
    /// Optional second routing root that only contains `route.rs` handlers.
    pub directory: Option<PathBuf>,
    /// URL prefix under which `api.directory` is mounted.
    pub prefix: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self { directory: None, prefix: "/api".into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Maximum buffered request body size in bytes.
    pub body_limit: usize,
    /// Seconds allowed for reading request headers (slowloris protection).
    pub header_timeout: u64,
    /// Seconds after which an in-flight request is aborted (0 = never).
    pub request_timeout: u64,
    /// Seconds to wait for in-flight requests on shutdown.
    pub shutdown_timeout: u64,
    pub compression: bool,
    pub http2: bool,
    /// Trust `X-Forwarded-For` / `X-Forwarded-Proto` from a reverse proxy.
    pub trust_proxy: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 3000,
            body_limit: 2 * 1024 * 1024,
            header_timeout: 10,
            request_timeout: 60,
            shutdown_timeout: 10,
            compression: true,
            http2: true,
            trust_proxy: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BuildConfig {
    pub output: PathBuf,
    /// Reserved: static generation currently renders pages sequentially.
    pub concurrency: usize,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self { output: PathBuf::from(".next-rust"), concurrency: 8 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AssetsConfig {
    /// Reserved: CSS is always minified at compile time by the CSS macros.
    pub optimize: bool,
    /// `Cache-Control` max-age (seconds) for files from `public/`.
    pub public_max_age: u64,
}

impl Default for AssetsConfig {
    fn default() -> Self {
        Self { optimize: true, public_max_age: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RenderingConfig {
    pub default: RenderingMode,
    /// Stream HTML progressively when a route has `loading.rs` / suspense.
    pub streaming: bool,
    /// Default revalidation interval (seconds) for static pages. `None` = never.
    pub revalidate: Option<u64>,
    /// Client-side navigation for `Link!` (loads a ~4 KB gzipped script).
    pub client_navigation: bool,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self { default: RenderingMode::Auto, streaming: true, revalidate: None, client_navigation: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImagesConfig {
    /// Reserved for the image optimizer (`Image!` currently uses the same default widths).
    pub sizes: Vec<u32>,
    pub quality: u8,
    /// Seconds optimized images are cached by clients.
    pub max_age: u64,
}

impl Default for ImagesConfig {
    fn default() -> Self {
        Self { sizes: vec![640, 750, 828, 1080, 1200, 1920, 2048, 3840], quality: 75, max_age: 60 * 60 * 24 * 30 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SecurityConfig {
    /// Send a conservative set of security headers on every response.
    pub headers: bool,
    /// `Content-Security-Policy` value. `{nonce}` is replaced by the per-request nonce.
    pub csp: Option<String>,
    /// CSRF protection for server actions.
    pub csrf: CsrfMode,
    /// `Strict-Transport-Security` max-age in seconds (0 disables the header).
    pub hsts_max_age: u64,
    /// Additional origins allowed to invoke server actions.
    pub allowed_origins: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self { headers: true, csp: None, csrf: CsrfMode::Origin, hsts_max_age: 0, allowed_origins: Vec::new() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CsrfMode {
    /// Verify `Origin`/`Sec-Fetch-Site` against the request host (default).
    #[default]
    Origin,
    /// Additionally require a double-submit token.
    Token,
    /// Disable (not recommended).
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnvConfig {
    /// Only variables with this prefix may reach client-visible output.
    pub public_prefix: String,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self { public_prefix: "NEXT_RUST_PUBLIC_".into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DevConfig {
    /// File watcher polling interval in milliseconds.
    pub poll_interval: u64,
    /// Show the error overlay in the browser.
    pub overlay: bool,
    /// Extra paths (relative to the project root) that trigger rebuilds.
    pub watch: Vec<PathBuf>,
}

impl Default for DevConfig {
    fn default() -> Self {
        Self { poll_interval: 250, overlay: true, watch: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingConfig {
    pub format: LogFormat,
    /// `error`, `warn`, `info`, `debug` or `trace`.
    pub level: String,
    /// Log one line per request.
    pub requests: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self { format: LogFormat::Auto, level: "info".into(), requests: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// `pretty` in development, `json` in production.
    #[default]
    Auto,
    Pretty,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedirectRule {
    /// Path pattern: `/old/:slug`, `/blog/:path*`.
    pub source: String,
    /// Destination; may reference captured parameters (`/new/:slug`).
    pub destination: String,
    #[serde(default)]
    pub permanent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderRule {
    pub source: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io { path: PathBuf, error: std::io::Error },
    Parse { path: PathBuf, message: String },
    Ambiguous { dir: PathBuf },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io { path, error } => write!(f, "could not read {}: {error}", path.display()),
            ConfigError::Parse { path, message } => {
                write!(f, "invalid configuration in {}:\n{message}", path.display())
            }
            ConfigError::Ambiguous { dir } => write!(
                f,
                "both {TOML_FILE} and {JSON_FILE} exist in {}; keep only one (TOML is preferred)",
                dir.display()
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Discover and load the configuration starting at `start`.
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, ConfigError> {
        if let Ok(explicit) = std::env::var("NEXT_RUST_CONFIG") {
            return Self::load(explicit);
        }
        let start = absolutize(start.as_ref());
        for dir in start.ancestors() {
            let toml = dir.join(TOML_FILE);
            let json = dir.join(JSON_FILE);
            match (toml.is_file(), json.is_file()) {
                (true, true) => return Err(ConfigError::Ambiguous { dir: dir.to_path_buf() }),
                (true, false) => return Self::load(toml),
                (false, true) => return Self::load(json),
                _ => {}
            }
            // Stop at the Cargo package boundary if there is no config:
            // zero-config projects are rooted at their manifest.
            if dir.join("Cargo.toml").is_file() && dir.join("app").is_dir() {
                break;
            }
        }
        let mut config = Config { root: start, ..Config::default() };
        config.apply_env_overrides();
        Ok(config)
    }

    /// Load a specific file (`.toml` or `.json`).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = absolutize(path.as_ref());
        let text = std::fs::read_to_string(&path).map_err(|error| ConfigError::Io { path: path.clone(), error })?;
        let mut config = if path.extension().is_some_and(|e| e == "json") {
            Self::from_json_str(&text)
        } else {
            Self::from_toml_str(&text)
        }
        .map_err(|message| ConfigError::Parse { path: path.clone(), message })?;
        config.root = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        config.source = Some(path);
        config.apply_env_overrides();
        Ok(config)
    }

    pub fn from_toml_str(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    pub fn from_json_str(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    /// `PORT` and `HOST` (conventional on container platforms) override the file.
    pub fn apply_env_overrides(&mut self) {
        if let Some(port) = std::env::var("PORT").ok().and_then(|p| p.parse().ok()) {
            self.server.port = port;
        }
        if let Ok(host) = std::env::var("HOST")
            && !host.is_empty()
        {
            self.server.host = host;
        }
    }

    /// Resolve a path from the configuration against the project root.
    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() { normalize(path) } else { normalize(&self.root.join(path)) }
    }

    pub fn app_dir(&self) -> PathBuf {
        self.resolve(&self.app.directory)
    }

    pub fn public_dir(&self) -> PathBuf {
        self.resolve(&self.app.public)
    }

    pub fn api_dir(&self) -> Option<PathBuf> {
        self.api.directory.as_ref().map(|d| self.resolve(d))
    }

    pub fn output_dir(&self) -> PathBuf {
        self.resolve(&self.build.output)
    }

    /// Validate paths and values; returns human friendly diagnostics.
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let app = self.app_dir();
        if !app.exists() {
            out.push(
                Diagnostic::error("NR0001", "App directory not found")
                    .location(&app)
                    .message(format!(
                        "The routing directory configured as `app.directory = {:?}`\nresolves to {}, which does not exist.",
                        self.app.directory,
                        app.display()
                    ))
                    .help("create the directory or change `[app] directory` in next-rust.toml"),
            );
        } else if !app.is_dir() {
            out.push(
                Diagnostic::error("NR0002", "App directory is not a directory")
                    .location(&app)
                    .help("`[app] directory` must point to a directory"),
            );
        }
        if let Some(api) = self.api_dir() {
            if !api.is_dir() {
                out.push(Diagnostic::error("NR0003", "API directory not found").location(api));
            }
            if !self.api.prefix.starts_with('/') {
                out.push(
                    Diagnostic::error("NR0004", "API prefix must start with `/`")
                        .message(format!("found `api.prefix = {:?}`", self.api.prefix)),
                );
            }
        }
        if !self.app.base_path.is_empty() && (!self.app.base_path.starts_with('/') || self.app.base_path.ends_with('/'))
        {
            out.push(
                Diagnostic::error("NR0005", "Invalid base path")
                    .message(format!("`app.base_path = {:?}`", self.app.base_path))
                    .help("use a leading slash and no trailing slash, e.g. `/docs`"),
            );
        }
        for rule in &self.redirects {
            if !rule.source.starts_with('/') {
                out.push(
                    Diagnostic::error("NR0006", "Redirect source must start with `/`")
                        .message(format!("source = {:?}", rule.source)),
                );
            }
        }
        if !matches!(self.logging.level.as_str(), "error" | "warn" | "info" | "debug" | "trace") {
            out.push(
                Diagnostic::warning("NR0007", "Unknown log level")
                    .message(format!("`logging.level = {:?}`; falling back to `info`", self.logging.level)),
            );
        }
        out
    }

    pub fn socket_addr_string(&self) -> String {
        if self.server.host.contains(':') {
            format!("[{}]:{}", self.server.host, self.server.port)
        } else {
            format!("{}:{}", self.server.host, self.server.port)
        }
    }
}

fn absolutize(p: &Path) -> PathBuf {
    if p.is_absolute() {
        normalize(p)
    } else {
        normalize(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(p))
    }
}

/// Lexically normalize a path: removes `.` and resolves `..` without
/// touching the filesystem (so it works for paths that do not exist yet).
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() { PathBuf::from(".") } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_zero_config() {
        let c = Config::from_toml_str("").unwrap();
        assert_eq!(c.app.directory, PathBuf::from("app"));
        assert_eq!(c.server.port, 3000);
        assert_eq!(c.build.output, PathBuf::from(".next-rust"));
        assert_eq!(c.rendering.default, RenderingMode::Auto);
    }

    #[test]
    fn parses_sections() {
        let c = Config::from_toml_str(
            r#"
            [app]
            directory = "src/web"
            [server]
            port = 8080
            [rendering]
            default = "server"
            [[redirects]]
            source = "/old"
            destination = "/new"
            permanent = true
            [plugins.analytics]
            id = "x"
            "#,
        )
        .unwrap();
        assert_eq!(c.app.directory, PathBuf::from("src/web"));
        assert_eq!(c.server.port, 8080);
        assert_eq!(c.rendering.default, RenderingMode::Dynamic);
        assert!(c.redirects[0].permanent);
        assert_eq!(c.plugins["analytics"]["id"], "x");
    }

    #[test]
    fn rejects_unknown_keys() {
        let err = Config::from_toml_str("[app]\ndirectroy = \"x\"").unwrap_err();
        assert!(err.contains("directroy"), "{err}");
    }

    #[test]
    fn json_is_supported() {
        let c = Config::from_json_str(r#"{"app":{"directory":"frontend"}}"#).unwrap();
        assert_eq!(c.app.directory, PathBuf::from("frontend"));
    }

    #[test]
    fn resolves_relative_to_root() {
        let c = Config {
            root: PathBuf::from("/work/repo/apps/site"),
            app: AppConfig { directory: PathBuf::from("../shared/my-pages"), ..Default::default() },
            ..Default::default()
        };
        assert_eq!(c.app_dir(), PathBuf::from("/work/repo/apps/shared/my-pages"));
        let c = Config {
            root: PathBuf::from("/work"),
            app: AppConfig { directory: PathBuf::from("/abs/pages/./x/.."), ..Default::default() },
            ..Default::default()
        };
        assert_eq!(c.app_dir(), PathBuf::from("/abs/pages"));
    }

    #[test]
    fn discovery_walks_up() {
        let base = std::env::temp_dir().join(format!("nr-config-{}", std::process::id()));
        let nested = base.join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(base.join(TOML_FILE), "[app]\ndirectory = \"website/routes\"\n").unwrap();
        let c = Config::discover(&nested).unwrap();
        assert_eq!(c.root, base);
        assert_eq!(c.app_dir(), base.join("website/routes"));
        let diags = c.validate();
        assert_eq!(diags[0].code, "NR0001");
        std::fs::write(base.join(JSON_FILE), "{}").unwrap();
        assert!(matches!(Config::discover(&nested), Err(ConfigError::Ambiguous { .. })));
        std::fs::remove_dir_all(&base).unwrap();
    }
}
