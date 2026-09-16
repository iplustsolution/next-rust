//! Minimal structured logging.
//!
//! Log lines go to stderr as human readable text in development and as one
//! JSON object per line in production (configurable). With the `tracing`
//! feature, events are emitted through `tracing` instead so any subscriber
//! (OpenTelemetry, Datadog, ...) can collect them.

use std::sync::atomic::{AtomicU8, Ordering};

static FORMAT: AtomicU8 = AtomicU8::new(0); // 0 pretty, 1 json
static LEVEL: AtomicU8 = AtomicU8::new(2); // 0 error, 1 warn, 2 info, 3 debug, 4 trace

pub fn configure(json: bool, level: &str) {
    FORMAT.store(u8::from(json), Ordering::Relaxed);
    let lvl = match level {
        "error" => 0,
        "warn" => 1,
        "debug" => 3,
        "trace" => 4,
        _ => 2,
    };
    LEVEL.store(lvl, Ordering::Relaxed);
}

pub fn is_json() -> bool {
    FORMAT.load(Ordering::Relaxed) == 1
}

fn enabled(level: u8) -> bool {
    level <= LEVEL.load(Ordering::Relaxed)
}

fn emit(level: u8, label: &str, msg: &str, fields: &[(&str, String)]) {
    if !enabled(level) {
        return;
    }
    #[cfg(feature = "tracing")]
    {
        let rendered: String = fields.iter().map(|(k, v)| format!(" {k}={v}")).collect();
        match level {
            0 => tracing::error!("{msg}{rendered}"),
            1 => tracing::warn!("{msg}{rendered}"),
            2 => tracing::info!("{msg}{rendered}"),
            _ => tracing::debug!("{msg}{rendered}"),
        }
        let _ = label;
    }
    #[cfg(not(feature = "tracing"))]
    if is_json() {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
        let mut obj = serde_json::Map::new();
        obj.insert("ts".into(), (ts as u64).into());
        obj.insert("level".into(), label.into());
        obj.insert("msg".into(), msg.into());
        for (k, v) in fields {
            obj.insert((*k).into(), v.clone().into());
        }
        eprintln!("{}", serde_json::Value::Object(obj));
    } else {
        let color = match level {
            0 => "\x1b[31m",
            1 => "\x1b[33m",
            _ => "\x1b[2m",
        };
        let rendered: String = fields.iter().map(|(k, v)| format!(" {k}={v}")).collect();
        if std::env::var_os("NO_COLOR").is_some() {
            eprintln!("{label:>5} {msg}{rendered}");
        } else {
            eprintln!("{color}{label:>5}\x1b[0m {msg}{rendered}");
        }
    }
}

pub fn error(msg: &str) {
    emit(0, "error", msg, &[]);
}

pub fn warn(msg: &str) {
    emit(1, "warn", msg, &[]);
}

pub fn info(msg: &str) {
    emit(2, "info", msg, &[]);
}

pub fn debug(msg: &str) {
    emit(3, "debug", msg, &[]);
}

/// Request log line.
pub fn request(method: &str, path: &str, status: u16, millis: f64, request_id: Option<&str>) {
    if !enabled(2) {
        return;
    }
    if is_json() || cfg!(feature = "tracing") {
        let mut fields = vec![
            ("method", method.to_owned()),
            ("path", path.to_owned()),
            ("status", status.to_string()),
            ("duration_ms", format!("{millis:.2}")),
        ];
        if let Some(id) = request_id {
            fields.push(("request_id", id.to_owned()));
        }
        emit(2, "info", "request", &fields);
    } else {
        let color = match status {
            500.. => "\x1b[31m",
            400..=499 => "\x1b[33m",
            300..=399 => "\x1b[36m",
            _ => "\x1b[32m",
        };
        if std::env::var_os("NO_COLOR").is_some() {
            eprintln!(" {method:<6} {path} {status} {millis:.1}ms");
        } else {
            eprintln!(" \x1b[1m{method:<6}\x1b[0m {path} {color}{status}\x1b[0m \x1b[2m{millis:.1}ms\x1b[0m");
        }
    }
}
