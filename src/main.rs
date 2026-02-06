#![allow(unused)]

mod attachments;
mod event;
mod logging;
mod setup;

use std::{
    thread::current,
    time::{Duration, SystemTime},
};

use opentelemetry::{
    Context, InstrumentationScope, InstrumentationScopeBuilder,
    global::{self, BoxedSpan, BoxedTracer, ObjectSafeSpan, ObjectSafeTracer},
    logs::{LogRecord, Logger, LoggerProvider},
    trace::{
        FutureExt, Span, SpanBuilder, SpanContext, TraceContextExt, Tracer, TracerProvider,
        noop::NoopSpan,
    },
};
use opentelemetry_sdk::logs::{SdkLogRecord, SdkLogger};
use rootcause::{
    Report, handlers,
    hooks::{Hooks, builtin_hooks::location::Location},
    markers::Mutable,
    report,
};
use rootcause_backtrace::{Backtrace, BacktraceCollector};
use tokio;

use crate::{attachments::*, event::*, setup::*};

#[tokio::main]
async fn main() -> Result<(), Report> {
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .report_creation_hook(OTelMetadataCollector)
        .install()
        .expect("Failed to install Rootcause hooks");

    let (trace_system, logs_system) = setup_system();
    let scope = InstrumentationScope::builder("otel-example").build();
    let logger = logs_system.logger_with_scope(scope.clone());
    let tracer = global::tracer_with_scope(scope.clone());

    {
        let _ctx = Context::current()
            .with_span(tracer.start("global-span"))
            .with_value(logger)
            .with_value(tracer)
            .attach();

        do_report_things().with_current_context().await?;
    }

    logs_system.force_flush();
    trace_system.force_flush();
    logs_system.shutdown()?;
    trace_system.shutdown()?;
    Ok(())
}

async fn do_report_things() -> Result<(), Report> {
    eprintln!("Context in do_report_things {:?}", Context::current());

    Context::current().span().add_event("Test", vec![]);

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut rep = {
        let _ctx = if let Some(tracer) = Context::current().get::<BoxedTracer>() {
            Context::current()
                .with_span(tracer.start("nested-span"))
                .attach()
        } else {
            Context::current().attach()
        };

        create_report().with_current_context().await
    };

    tokio::time::sleep(Duration::from_millis(100)).await;

    Context::current().span().record_error_report_event(&rep);

    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}

async fn create_report() -> Report {
    Context::current().span().add_event("Test 2", vec![]);

    report!("Something went wrong!")
}
