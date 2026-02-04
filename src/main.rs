#![allow(unused)]

mod attachments;
mod event;
mod setup;

use std::thread::current;

use opentelemetry::{
    Context,
    global::{self, ObjectSafeSpan, ObjectSafeTracerProvider},
    trace::{FutureExt, Span, SpanContext, TraceContextExt, Tracer},
};
use rootcause::{Report, handlers, hooks::Hooks, markers::Mutable, report};
use rootcause_backtrace::{Backtrace, BacktraceCollector};
use tokio;

use crate::{attachments::*, event::*, setup::*};

#[tokio::main]
async fn main() -> Result<(), Report> {
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .report_creation_hook(ReportOTelSpans)
        .attachment_collector(ReportTimestamps)
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
    let ctx = Context::current().attach();

    let mut rep = report!("Something went wrong!");

    rep.record_with_current_span();
    eprintln!("{}", rep);

    Ok(())
}
