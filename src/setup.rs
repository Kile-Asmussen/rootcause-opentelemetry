
use std::{any::TypeId, sync::LazyLock, time::SystemTime};
use rootcause::{Report, handlers, hooks::{Hooks, report_creation::AttachmentCollector}};
use tokio;

use opentelemetry::{InstrumentationScope, StringValue, global};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};

pub(crate) fn trace_system() -> SdkTracerProvider {
    let exporter = opentelemetry_stdout::SpanExporter::default();
    let resource = Resource::builder()
        .with_service_name("rootcause-opentelemetry")
        .build();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_resource(resource)
        .build();
    global::set_tracer_provider(provider.clone());
    provider
}