//! `.env` loading with an explicit server/public split.
//!
//! Files are loaded in increasing priority:
//!
//! 1. `.env`
//! 2. `.env.{environment}` (e.g. `.env.production`)
//! 3. `.env.local` (skipped when environment is `test`)
//! 4. `.env.{environment}.local`
//!
//! Variables already present in the process environment always win, so
//! platform-provided secrets are never overwritten by files.
//!
//! Only variables whose name starts with the public prefix (by default
//! `NEXT_RUST_PUBLIC_`) are returned by [`EnvVars::public`]. Nothing else is
//! ever made available to client-visible output.

use std::collections::BTreeMap;
use std::path::Path;

use crate::mode::Environment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvVars {
    vars: BTreeMap<String, String>,
    public_prefix: String,
}

impl EnvVars {
    /// Read `.env` files from `root` without modifying the process environment.
    pub fn load(root: &Path, environment: Environment, public_prefix: &str) -> std::io::Result<Self> {
        let env = environment.as_str();
        let mut files = vec![".env".to_owned(), format!(".env.{env}")];
        if environment != Environment::Test {
            files.push(".env.local".to_owned());
        }
        files.push(format!(".env.{env}.local"));

        let mut vars = BTreeMap::new();
        for file in files {
            let path = root.join(&file);
            match std::fs::read_to_string(&path) {
                Ok(text) => vars.extend(parse(&text)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        for (k, v) in vars.iter_mut() {
            if let Ok(existing) = std::env::var(k) {
                *v = existing;
            }
        }
        Ok(Self { vars, public_prefix: public_prefix.to_owned() })
    }

    /// Export loaded values into the process environment (only for keys that
    /// are not already set). Call once at startup before spawning threads.
    pub fn apply_to_process(&self) {
        for (k, v) in &self.vars {
            if std::env::var_os(k).is_none() {
                // SAFETY: called during single-threaded startup, see docs.
                unsafe_set_var(k, v);
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Variables that are safe to expose to clients.
    pub fn public(&self) -> BTreeMap<String, String> {
        let mut out: BTreeMap<String, String> = self
            .vars
            .iter()
            .filter(|(k, _)| k.starts_with(&self.public_prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (k, v) in std::env::vars() {
            if k.starts_with(&self.public_prefix) {
                out.insert(k, v);
            }
        }
        out
    }

    pub fn is_public(&self, key: &str) -> bool {
        key.starts_with(&self.public_prefix)
    }
}

#[allow(unsafe_code)]
fn unsafe_set_var(k: &str, v: &str) {
    // `set_var` is `unsafe` since edition 2024 because it races with other
    // threads reading the environment. The framework only calls this from
    // `main` before the async runtime starts.
    unsafe { std::env::set_var(k, v) }
}

/// Parse dotenv syntax.
///
/// Supports `KEY=value`, `export KEY=value`, `# comments`, single quotes
/// (literal), double quotes (with `\n`, `\t`, `\"`, `\\` escapes and
/// multi-line values) and inline comments after unquoted values.
pub fn parse(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut lines = text.lines();
    while let Some(raw) = lines.next() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
            continue;
        }
        let value = value.trim_start();
        let parsed = if let Some(rest) = value.strip_prefix('"') {
            let mut buf = rest.to_owned();
            // Multi-line double-quoted values.
            while !closes_double_quote(&buf) {
                match lines.next() {
                    Some(next) => {
                        buf.push('\n');
                        buf.push_str(next);
                    }
                    None => break,
                }
            }
            unescape_double(&buf)
        } else if let Some(rest) = value.strip_prefix('\'') {
            rest.split_once('\'').map(|(v, _)| v.to_owned()).unwrap_or_else(|| rest.to_owned())
        } else {
            let v = match value.find(" #") {
                Some(i) => &value[..i],
                None => value,
            };
            v.trim().to_owned()
        };
        out.insert(key.to_owned(), parsed);
    }
    out
}

fn closes_double_quote(s: &str) -> bool {
    let mut escaped = false;
    for c in s.chars() {
        match c {
            '\\' if !escaped => escaped = true,
            '"' if !escaped => return true,
            _ => escaped = false,
        }
    }
    false
}

fn unescape_double(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(other) => out.push(other),
                None => {}
            },
            '"' => break,
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotenv() {
        let vars = parse(
            "# comment\nA=1\nexport B = two # trailing\nC=\"line\\nnext\"\nD='lit $x \\n'\nE=\"multi\nline\"\nbad line\n",
        );
        assert_eq!(vars["A"], "1");
        assert_eq!(vars["B"], "two");
        assert_eq!(vars["C"], "line\nnext");
        assert_eq!(vars["D"], "lit $x \\n");
        assert_eq!(vars["E"], "multi\nline");
        assert_eq!(vars.len(), 5);
    }

    #[test]
    fn layering_and_public_split() {
        let dir = std::env::temp_dir().join(format!("nr-env-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".env"), "DATABASE_URL=base\nNEXT_RUST_PUBLIC_SITE=a\n").unwrap();
        std::fs::write(dir.join(".env.production"), "DATABASE_URL=prod\n").unwrap();
        std::fs::write(dir.join(".env.local"), "NEXT_RUST_PUBLIC_SITE=local\n").unwrap();
        let env = EnvVars::load(&dir, Environment::Production, "NEXT_RUST_PUBLIC_").unwrap();
        assert_eq!(env.get("DATABASE_URL"), Some("prod"));
        let public = env.public();
        assert_eq!(public.get("NEXT_RUST_PUBLIC_SITE").map(String::as_str), Some("local"));
        assert!(!public.contains_key("DATABASE_URL"));
        // .env.local is ignored for tests
        let env = EnvVars::load(&dir, Environment::Test, "NEXT_RUST_PUBLIC_").unwrap();
        assert_eq!(env.get("NEXT_RUST_PUBLIC_SITE"), Some("a"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
