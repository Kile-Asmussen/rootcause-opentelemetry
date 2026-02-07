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
    Context, ContextGuard, InstrumentationScope, InstrumentationScopeBuilder, KeyValue,
    global::{self, BoxedSpan, BoxedTracer, ObjectSafeSpan, ObjectSafeTracer},
    logs::{LogRecord, Logger, LoggerProvider},
    metrics::MeterProvider,
    trace::{
        FutureExt, Span, SpanBuilder, SpanContext, Status, TraceContextExt, Tracer, TracerProvider,
        noop::NoopSpan,
    },
};
use opentelemetry_sdk::{
    logs::{SdkLogRecord, SdkLogger},
    trace::{self as trace_sdk, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::attribute;
use rootcause::{Report, handlers, hooks::Hooks, markers::SendSync, prelude::*, report};
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

    let (trace_system, logs_system, metrics_system) = setup_system();
    let scope = InstrumentationScope::builder("otel-example").build();
    let logger = logs_system.logger_with_scope(scope.clone());
    let tracer = trace_system.tracer_with_scope(scope.clone());
    let meter = metrics_system.meter_with_scope(scope.clone());

    let context = Context::new()
        .with_value(logger)
        .with_value(tracer)
        .with_value(meter);

    run()
        .with_context(context.enter_span("run", vec![]))
        .await
        .map_err(|rep| {
            eprintln!("{}", rep);
        });

    logs_system.force_flush();
    trace_system.force_flush();
    metrics_system.force_flush();

    [
        logs_system.shutdown(),
        trace_system.shutdown(),
        metrics_system.shutdown(),
    ]
    .into_iter()
    .collect_reports_vec::<SendSync>()
    .context("Shutdown failed")
    .map_err(|rep| eprintln!("{}", rep));

    Ok(())
}

fn new_span(ctx: &Context, name: &'static str) -> trace_sdk::Span {
    ctx.get::<<SdkTracerProvider as TracerProvider>::Tracer>()
        .expect("No tracing was configured")
        .start_with_context(name, ctx)
}

trait ContextExt {
    fn new_span(&self, name: &'static str, attributes: Vec<KeyValue>) -> trace_sdk::Span;
    fn enter_span(&self, name: &'static str, attributes: Vec<KeyValue>) -> Self;
}

impl ContextExt for Context {
    fn enter_span(&self, name: &'static str, attributes: Vec<KeyValue>) -> Context {
        self.with_span(self.new_span(name, attributes))
    }

    fn new_span(&self, name: &'static str, attributes: Vec<KeyValue>) -> trace_sdk::Span {
        self.get::<<SdkTracerProvider as TracerProvider>::Tracer>()
            .expect("No tracing was configured")
            .start_with_context(name, self)
    }
}

async fn run() -> Result<(), Report> {
    let ctx = Context::current();

    tokio::time::sleep(Duration::from_millis(100)).await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    inner()
        .with_context(ctx.enter_span("inner", vec![]))
        .await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    ctx.span().set_status(Status::Ok);

    ctx.span().end();

    Ok(())
}

async fn inner() -> Result<(), Report> {
    let ctx = Context::current();

    ctx.span().add_event("something.happened.2", vec![]);

    tokio::time::sleep(Duration::from_millis(100)).await;

    ctx.span().set_status(Status::Ok);

    ctx.span().end();
    Ok(())
}

async fn do_report_things() -> Result<(), Report> {
    Context::current().span().add_event("Test", vec![]);

    tokio::time::sleep(Duration::from_millis(100)).await;

    {
        let ctx = if let Some(tracer) =
            Context::current().get::<<SdkTracerProvider as TracerProvider>::Tracer>()
        {
            Context::current().with_span(tracer.start("nested-span"))
        } else {
            Context::current()
        }
        .attach();

        create_report().with_current_context().await;
    };

    Ok(())
}

async fn create_report() {
    Context::current().span().add_event("Test 2", vec![]);
}
