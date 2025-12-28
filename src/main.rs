
use core::time;
use std::{alloc::System, any::{Any, TypeId}, backtrace, sync::LazyLock, time::SystemTime};
use rootcause::{Report, ReportRef, handlers, hooks::{Hooks, report_creation::{AttachmentCollector, ReportCreationHook}}, markers::{Dynamic, ObjectMarkerFor}, report_attachment::ReportAttachment, report_attachments::{ReportAttachments, ReportAttachmentsIter}};
use rootcause_backtrace::Backtrace;
use tokio;

use opentelemetry::{Array, Context, InstrumentationScope, KeyValue, StringValue, Value, global, trace::{self, Span, SpanRef, Status, TraceContextExt}};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};

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

#[tokio::main]
async fn main() -> Result<(), Report> {
    Hooks::new()
        .attachment_collector(SystemTimeCollector)
        .install()
        .expect("Failed to install hooks");

    let trace_system = trace_system();



    trace_system.shutdown()?;
    Ok(())
}

struct SystemTimeCollector;
impl AttachmentCollector<SystemTime> for SystemTimeCollector {
    type Handler = handlers::Debug;

    fn collect(&self) -> SystemTime {
        SystemTime::now()
    }
}

trait ReportExt: Sized {
    fn otel(self) -> impl OTelConcreteBuilder {
        self.otel_raw()
            .timestamp()
            .backtrace()
            .attachments()
    }
    fn otel_raw(self) -> impl OTelConcreteBuilder;
}

struct OTelReportEventBuilder<C, O, T>
where
    C : 'static + ?Sized,
    O : 'static,
    T : 'static
{
    report: Report<C, O, T>,
    timestamp: Option<SystemTime>,
    status: trace::Status,
    attachments: Vec<StringValue>,
    backtrace: Option<StringValue>,
    message: StringValue,
    ex_type: Option<StringValue>,
}

#[derive(Default, Debug, PartialEq)]
struct OTelReportBuilderActions {
    ex_type: Trinary<StringValue>,
    message: Option<StringValue>,
    timestamp: Trinary<Option<SystemTime>>,
    backtrace: bool,
    escaped: Option<bool>,
    attachments: Vec<AttachmentAction>,
    children: Trinary<Box<OTelReportBuilderActions>>
}


impl OTelReportBuilderActions {
    fn apply<O: OTelAbstractBuilder>(self, mut other: O) -> O {
        
        other = match self.ex_type {
            Trinary::Default => other
            Trinary::Inferred => other.ex_type(),
            Trinary::Specific(s) => other.set_ex_type(s),
        };

        other = match self.timestamp {
            Trinary::Default => other,
            Trinary::Inferred => other.timestamped(),
            Trinary::Specific(None) => other.timestamp_now(),
            Trinary::Specific(Some(st)) => other.set_timestamp(st),
        };

        other = if self.backtrace {
            other.backtrace()
        } else {
            other
        };

        other = match self.escaped {
            Some(has_escaped) => other.escaped(has_escaped),
            _ => other
        };

        for at in self.attachments {
            other = match at {
                AttachmentAction::Smart() => other.attachments(),
                AttachmentAction::All() => other.all_attachments(),
                AttachmentAction::Custom(string_value) => other.add_attacment(string_value),
                AttachmentAction::OfType(type_id) => other.attachments_of_type_id(type_id),
            }
        }

        other = match self.children {
            Trinary::Default => other,
            Trinary::Inferred => other.recurse(),
            Trinary::Specific(builder) => other.children(|_| *builder),
        };

        other
    }
}

#[derive(Default, Debug, PartialEq)]
enum Trinary<T> {
    #[default]
    Default,
    Inferred,
    Specific(T)
}

#[derive(Debug, PartialEq)]
enum AttachmentAction {
    Smart(),
    All(),
    Custom(StringValue),
    OfType(TypeId),
}

pub trait OTelAbstractBuilder: Sized {
    fn ex_type(self) -> Self;
    fn set_ex_type(self, ex_type: impl Into<StringValue>) -> Self;

    fn message(self, msg: impl Into<StringValue>) -> Self;

    fn timestamped(self) -> Self;
    fn set_timestamp(self, systime: SystemTime) -> Self;
    fn timestamp_now(self) -> Self;

    fn backtrace(self) -> Self;

    fn escaped(self, has_escaped: bool) -> Self;

    fn add_attacment(self, at: impl Into<StringValue>) -> Self;
    fn all_attachments(self) -> Self;
    fn attachments(self) -> Self;
    fn attachments_of_type_id(self, type_id: TypeId) -> Self;
    fn attachments_of_type<T: 'static>(self) -> Self {
        self.attachments_of_type_id(TypeId::of::<T>())
    }
    
    fn recurse(self) -> Self;
    fn children<F>(self, f: F) -> Self
        where F: FnOnce(OTelReportBuilderActions) -> OTelReportBuilderActions;
}

impl OTelAbstractBuilder for OTelReportBuilderActions {
    fn escaped(mut self, has_escaped: bool) -> Self {
        self.escaped = Some(has_escaped);
        self
    }

    fn ex_type(mut self) -> Self {
        self.ex_type = Trinary::Inferred;
        self
    }

    fn set_ex_type(mut self, ex_type: impl Into<StringValue>) -> Self {
        self.ex_type = Trinary::Specific(ex_type.into());
        self
    }

    fn backtrace(mut self) -> Self {
        self.backtrace = true;
        self
    }

    fn timestamped(mut self) -> Self {
        self.timestamp = Trinary::Inferred;
        self
    }

    fn set_timestamp(mut self, systime: SystemTime) -> Self {
        self.timestamp = Trinary::Specific(Some(systime));
        self
    }

    fn timestamp_now(mut self) -> Self {
        self.timestamp = Trinary::Specific(None);
        self
    }

    fn recurse(mut self) -> Self {
        self.children = Trinary::Inferred;
        self
    }

    fn add_attacment(mut self, at: impl Into<StringValue>) -> Self {
        self.attachments.push(AttachmentAction::Custom(at.into()));
        self
    }

    fn all_attachments(mut self) -> Self {
        self.attachments.push(AttachmentAction::All());
        self
    }

    fn attachments(mut self) -> Self {
        self.attachments.push(AttachmentAction::Smart());
        self
    }

    fn attachments_of_type_id(mut self, type_id: TypeId) -> Self {
        self.attachments.push(AttachmentAction::OfType(type_id));
        self
    }

    fn children<F>(mut self, f: F) -> Self
        where F: FnOnce(OTelReportBuilderActions) -> OTelReportBuilderActions {
        self.children = Trinary::Specific(Box::new(f(OTelReportBuilderActions::default())));
        self
    }
}



pub trait OTelConcreteBuilder: OTelAbstractBuilder {
    type Context: 'static;
    type ObjectMarker: ObjectMarkerFor<Self::Context> + 'static;
    type ThreadSafety: 'static;

    type WrappedReport;

    fn apply(self, actions: OTelReportBuilderActions) -> Self {

    }
    
    fn send(self) -> Self::WrappedReport;

    fn underlying_report_ref(&self)
        -> Option<ReportRef<Self::Context, Self::ObjectMarker, Self::ThreadSafety>>;

    fn get_context_typename(&self) -> &'static str {
        std::any::type_name::<Self::Context>()
    }

    fn specify_ex_type(self, ex_type: impl Into<StringValue>) -> Self;

    fn ex_type(self) -> Self {
        let ex_type = self.get_context_typename();
        self.specify_ex_type(ex_type)
    }

    fn children<F>(self, f: F) -> Self
        where F: FnOnce(OTelReportBuilderActions) -> OTelReportBuilderActions;

    fn timestamped(self) -> Self {
        let timestamp =
            self.underlying_report_ref()
            .and_then(
                |report|
                report.attachments()
                .iter().find_map(|r| r.downcast_inner::<SystemTime>())
            ).map(|ts| *ts);

        if let Some(timestamp) = timestamp {
            self.set_timestamp(timestamp)
        } else {
            self
        }
    }

    fn timestamp_now(self) -> Self {
        self.set_timestamp(SystemTime::now())
    }

    fn backtrace_override(self, backtrace: impl Into<StringValue>) -> Self;

    fn backtrace(self) -> Self {
        let backtrace = 
            self.underlying_report_ref()
            .and_then(
                |report|
                report.attachments()
                .iter().find_map(|r| r.downcast_attachment::<Backtrace>())
            ).map(|bt| bt.format_inner().to_string()); 
        
        if let Some(backtrace) = backtrace {
            self.backtrace_override(backtrace)
        } else {
            self
        }
    }

    fn append_attacments(mut self, attachments: impl IntoIterator<Item = impl Into<StringValue>>) -> Self {
        for at in attachments {
            self = self.add_attacment(at);
        }
        self
    }

    fn attachments(self) -> Self {
        let attachments: Option<Vec<String>> =
            self.underlying_report_ref()
            .map(
                |report|
                report.attachments().iter().filter(
                    |at|
                        at.inner_type_id() != TypeId::of::<Backtrace>()
                        && at.inner_type_id() != TypeId::of::<Backtrace>()
                ).map(
                    |at| at.format_inner().to_string()
                ).collect::<Vec<_>>()
            );
        
        if let Some(attachments) = attachments {
            self.append_attacments(attachments)
        } else {
            self
        }
    }

    fn all_attachments(self) -> Self {
        let attachments: Option<Vec<String>> =
            self.underlying_report_ref()
            .map(
                |report|
                report.attachments().iter()
                .map(|at| at.format_inner().to_string())
                .collect::<Vec<_>>()
            );
        
        if let Some(attachments) = attachments {
            self.append_attacments(attachments)
        } else {
            self
        }
    }

    fn attachments_of_type<T>(self) -> Self {
        let attachments: Option<Vec<String>> =
            self.underlying_report_ref()
            .map(
                |report|
                report.attachments().iter()
                .filter( |at| at.inner_type_id() == TypeId::of::<T>())
                .map(|at| at.format_inner().to_string())
                .collect::<Vec<_>>()
            );
        if let Some(attachments) = attachments {
            self.append_attacments(attachments)
        } else {
            self
        }
    }
}

trait SpanExt {
    fn report_error_ref<T, C, A>(self, rep: ReportRef<T, C, A>, escaped: Option<bool>)
        where 
            T : 'static + ?Sized,
            C : 'static,
            A : 'static;
}

impl<'a> SpanExt for SpanRef<'a> {
    fn report_error_ref<T, C, A>(self, rep: ReportRef<T, C, A>, escaped: Option<bool>)
        where 
            T : 'static + ?Sized,
            C : 'static,
            A : 'static
    {
        
    }
}

impl<S: Span> SpanExt for &mut S {
    fn report_error_ref<T, C, A>(self, rep: ReportRef<T, C, A>, escaped: Option<bool>)
        where 
            T : 'static + ?Sized,
            C : 'static,
            A : 'static
    {

        let mut attributes = vec![
            KeyValue::new("exception.type", std::any::type_name::<T>()),
            KeyValue::new("exception.message", rep.format_current_context().to_string()),
        ];

        if let Some(escaped) = escaped {
            attributes.push(
                KeyValue::new("exception.escaped", escaped)
            );
        }

        let mut attachments = vec![];
        let mut timestamp = None;
        let mut backtrace : Option<String> = None;

        for atch in rep.attachments() {
            if let Some(btc) = atch.downcast_attachment::<Backtrace>() && backtrace.is_none() {
                backtrace = Some(btc.to_string());
            } else if let Some(ts) = atch.downcast_inner::<SystemTime>() && timestamp.is_none() {
                timestamp = Some(*ts);
            } else {
                attachments.push(StringValue::from(atch.to_string()))
            }
        }

        if let Some(backtrace) = backtrace {
            attributes.push(KeyValue::new("exception.stacktrace", backtrace));
        }

        if attachments.len() > 0 {
            attributes.push(KeyValue::new("attachments",
                Value::Array(Array::String(attachments))));
        }

        if let Some(timestamp) = timestamp {
            self.add_event_with_timestamp("exception", timestamp, attributes);
        } else {
            self.add_event("exception", attributes);
        }
    }
}