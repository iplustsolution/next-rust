use next_rust::prelude::*;

pub async fn Page() -> Result<impl View> {
    Err::<Node, _>(Error::msg("report service unavailable"))
}
