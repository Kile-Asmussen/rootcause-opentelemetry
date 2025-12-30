
#![allow(unused)]

use opentelemetry::{global::{self, ObjectSafeSpan, ObjectSafeTracerProvider}, trace::{FutureExt, Tracer}};
use rootcause::{Report, handlers, hooks::Hooks, markers::Mutable, report};
use tokio;


use crate::{builder::ExceptionEventBuilder, reports::{ReportExt, SystemTimeCollector}, setup::*, spec::ExceptionEventSpec};

mod attachments;
mod builder;
mod reports;
mod setup;
mod span;
mod spec;

#[tokio::main]
async fn main() -> Result<(), Report> {
    Hooks::new()
        .attachment_collector(SystemTimeCollector)
        .install()
        .expect("Failed to install hooks");

    let trace_system = trace_system();
    
    let tracer = global::tracer("rootcause_opentelemetry");

    let mut span = tracer.start("do_report_things");

    do_report_things().with_current_context().await?;
    
    span.end();

    trace_system.shutdown()?;
    Ok(())
}

async fn do_report_things() -> Result<(), Report> {

    let rep = Report::<i32>::new_custom::<handlers::Debug>(10i32);

    rep.otel().send();

    Ok(())
}