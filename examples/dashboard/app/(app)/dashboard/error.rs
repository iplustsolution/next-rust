use next_rust::prelude::*;

pub fn ErrorBoundary(info: ErrorInfo) -> impl View {
    div![role("alert"), h2!["This section failed to load"], p![info.message], small![format!("ref {}", info.digest)]]
}
