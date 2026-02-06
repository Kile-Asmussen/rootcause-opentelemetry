use rootcause::{
    Report, handlers,
    hooks::{Hooks, report_creation::AttachmentCollector},
};
use std::{any::TypeId, sync::LazyLock, time::SystemTime};
use tokio;

use opentelemetry::{InstrumentationScope, StringValue, global};
use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider, trace::SdkTracerProvider};

pub(crate) fn setup_system() -> (SdkTracerProvider, SdkLoggerProvider) {
    let resource = Resource::builder()
        .with_service_name("rootcause-opentelemetry")
        .build();

    let span_exporter = opentelemetry_stdout::SpanExporter::default();

    let trace_provider = SdkTracerProvider::builder()
        .with_simple_exporter(span_exporter)
        .with_resource(resource.clone())
        .build();

    let log_exporter = opentelemetry_stdout::LogExporter::default();

    let logger_provider = SdkLoggerProvider::builder()
        .with_resource(resource.clone())
        .with_simple_exporter(log_exporter)
        .build();

    global::set_tracer_provider(trace_provider.clone());
    (trace_provider, logger_provider)
}

pub(crate) fn logging_system() -> SdkLoggerProvider {
    todo!()
}
