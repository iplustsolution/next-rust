use serde::{Deserialize, Serialize};

/// How a page is rendered.
///
/// | mode      | behaviour                                                        |
/// |-----------|------------------------------------------------------------------|
/// | `static`  | rendered at build time (optionally revalidated, see ISR)         |
/// | `dynamic` | rendered on every request (`server` is accepted as an alias)     |
/// | `auto`    | the build decides: static unless the route needs request data    |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderingMode {
    #[default]
    Auto,
    Static,
    #[serde(alias = "server")]
    Dynamic,
}

impl RenderingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RenderingMode::Auto => "auto",
            RenderingMode::Static => "static",
            RenderingMode::Dynamic => "dynamic",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "static" => Some(Self::Static),
            "dynamic" | "server" => Some(Self::Dynamic),
            _ => None,
        }
    }
}

/// Runtime environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Production,
    Test,
}

impl Environment {
    /// Read from `NEXT_RUST_ENV` (falls back to `production` for release
    /// builds and `development` for debug builds).
    pub fn from_env() -> Self {
        match std::env::var("NEXT_RUST_ENV").ok().as_deref() {
            Some("development" | "dev") => Self::Development,
            Some("production" | "prod") => Self::Production,
            Some("test") => Self::Test,
            _ if cfg!(debug_assertions) => Self::Development,
            _ => Self::Production,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Production => "production",
            Self::Test => "test",
        }
    }

    pub fn is_dev(self) -> bool {
        self == Self::Development
    }
}
