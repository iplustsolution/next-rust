use next_rust::prelude::*;

pub fn Loading() -> impl View {
    p![aria("busy", "true"), "Loading dashboard…"]
}
