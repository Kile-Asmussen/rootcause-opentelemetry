#![allow(unused)]

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
        self, FutureExt, Span, SpanBuilder, SpanContext, SpanKind, Status, TraceContextExt, Tracer,
        TracerProvider, noop::NoopSpan,
    },
};
use opentelemetry_sdk::{
    Resource,
    logs::{SdkLogRecord, SdkLogger, SdkLoggerProvider},
    trace::{self as trace_sdk, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::attribute;
use rootcause::{
    Report,
    handlers::{self, Display},
    hooks::Hooks,
    markers::SendSync,
    prelude::*,
    report,
};
use rootcause_backtrace::{Backtrace, BacktraceCollector};
use rootcause_opentelemetry::{
    attachments::{HideTraceAttachments, OpenTelemetryMetadataCollector},
    log_event::LoggerExt,
    span_event::SpanRefReportExt,
};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Report> {
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .report_creation_hook(OpenTelemetryMetadataCollector::new())
        .attachment_formatter(HideTraceAttachments)
        .install()
        .expect("Failed to install rootcause hooks");

    let (trace_system, logs_system) = setup_system();
    let scope = InstrumentationScope::builder("otel-example").build();

    let logger = logs_system.logger("otel-logger");
    let tracer = trace_system.tracer_with_scope(scope.clone());

    tracer.in_span("outer-span", |c| {
        let rep = tracer
            .in_span("inner-span-1", |_| report!("something bad happened!"))
            .context("more bad");

        logger.emit_error_report(&rep);

        tracer.in_span("inner-span-2", move |c| {
            c.span()
                .record_error_report(&rep.context("something else bad"))
                .link_child_report_spans()
                .as_event_brief()
                .with_error_status();
        });
    });

    logs_system.force_flush();
    trace_system.force_flush();

    [logs_system.shutdown(), trace_system.shutdown()]
        .into_iter()
        .collect_reports_vec::<SendSync>()
        .context("Shutdown failed")
        .map_err(|rep| eprintln!("{}", rep));

    Ok(())
}

pub(crate) fn setup_system() -> (SdkTracerProvider, SdkLoggerProvider) {
    let resource = Resource::builder()
        .with_service_name("rootcause-opentelemetry")
        .build();

    let span_exporter = opentelemetry_stdout::SpanExporter::default();

    let trace_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_simple_exporter(span_exporter)
        .build();

    let log_exporter = opentelemetry_stdout::LogExporter::default();

    let logger_provider = SdkLoggerProvider::builder()
        .with_resource(resource.clone())
        .with_simple_exporter(log_exporter)
        .build();

    (trace_provider, logger_provider)
}
