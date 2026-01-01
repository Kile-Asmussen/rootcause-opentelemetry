#![allow(unused)]

mod attachments;
mod event_builder;
mod setup;

use std::thread::current;

use opentelemetry::{
    Context,
    global::{self, ObjectSafeSpan, ObjectSafeTracerProvider},
    trace::{FutureExt, Span, TraceContextExt, Tracer},
};
use rootcause::{Report, handlers, hooks::Hooks, markers::Mutable, report};
use tokio;

use crate::{attachments::ReportTimestamping, setup::*};

#[tokio::main]
async fn main() -> Result<(), Report> {
    Hooks::new()
        .attachment_collector(ReportTimestamping)
        .install()
        .expect("Failed to install Rootcause hooks");

    let trace_system = trace_system();

    let tracer = global::tracer("rootcause_opentelemetry");
    {
        let _ctx = Context::current().with_span(tracer.start("test")).attach();

        do_report_things().with_current_context().await?;
    }

    trace_system.force_flush();
    trace_system.shutdown()?;
    Ok(())
}

async fn do_report_things() -> Result<(), Report> {
    let mut rep = Report::<i32>::new_custom::<handlers::Display>(10i32);

    rep = rep.attach("This is a test");

    let ctx = Context::current().attach();

    Ok(())
}
