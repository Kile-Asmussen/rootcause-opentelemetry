
use std::{any::TypeId, sync::LazyLock, time::SystemTime};
use rootcause::{Report, handlers, hooks::{Hooks, report_creation::AttachmentCollector}};
use tokio;

use opentelemetry::{InstrumentationScope, StringValue, global};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};

use crate::{builder::OTelEventBuilder, spec::OTelEventSpec};

mod builder;
mod spec;

static RESOURCE: LazyLock<Resource> = LazyLock::new(||
    Resource::builder()
        .with_service_name("rootcause-example")
        .build()
);

fn trace_system() -> SdkTracerProvider {
    let exporter = opentelemetry_stdout::SpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_resource(RESOURCE.clone())
        .build();
    global::set_tracer_provider(provider.clone());
    provider
}

static SCOPE: LazyLock<InstrumentationScope> = LazyLock::new(||
    InstrumentationScope::builder("rootcause-example-stdout")
        .with_version(env!("CARGO_PKG_VERSION"))
        .build()
);