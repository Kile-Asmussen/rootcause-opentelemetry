use opentelemetry_stdout::MetricExporter;
use rootcause::{
    Report, handlers,
    hooks::{Hooks, report_creation::AttachmentCollector},
};
use std::{any::TypeId, sync::LazyLock, time::SystemTime};
use tokio;

use opentelemetry::{InstrumentationScope, StringValue, global};
use opentelemetry_sdk::{
    Resource, logs::SdkLoggerProvider, metrics::SdkMeterProvider, trace::SdkTracerProvider,
};

pub(crate) fn setup_system() -> (SdkTracerProvider, SdkLoggerProvider, SdkMeterProvider) {
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

    let metric_exporter = MetricExporter::builder().build();

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_periodic_exporter(metric_exporter)
        .build();

    (trace_provider, logger_provider, meter_provider)
}
