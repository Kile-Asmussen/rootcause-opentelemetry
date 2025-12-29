
use rootcause::{Report, hooks::Hooks};
use tokio;


use crate::{reports::SystemTimeCollector, spec::OTelEventSpec};

mod builder;
mod spec;
mod reports;
mod span;

#[tokio::main]
async fn main() -> Result<(), Report> {
    Hooks::new()
        .attachment_collector(SystemTimeCollector)
        .install()
        .expect("Failed to install hooks");


    Ok(())
}