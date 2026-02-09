use std::time::SystemTime;

use opentelemetry::{
    Array, Context, Value,
    logs::{AnyValue, LogRecord, Logger, Severity},
    trace::{SpanContext, TraceContextExt},
};

use crate::utilities::{AsReportRef, AttachmentsExt, attributes, timestamp};

pub trait LoggerExt: Sized {
    fn emit_error_report(&self, rep: &impl AsReportRef);
}

impl<L: Logger + Sized> LoggerExt for L {
    fn emit_error_report(&self, rep: &impl AsReportRef) {
        let rep = rep.as_report_ref();
        let mut record = self.create_log_record();
        record.set_event_name("exception");
        record.set_observed_timestamp(timestamp(rep));
        record.set_timestamp(SystemTime::now());

        let severity = rep
            .find_attachment_inner()
            .cloned()
            .unwrap_or(Severity::Error);
        record.set_severity_number(severity);
        record.set_severity_text(severity.name());

        let span_context = rep
            .find_attachment_inner::<SpanContext>()
            .cloned()
            .unwrap_or_else(|| Context::current().span().span_context().clone());

        if span_context.is_valid() {
            record.set_trace_context(
                span_context.trace_id(),
                span_context.span_id(),
                Some(span_context.trace_flags()),
            );
        }

        for kv in attributes(rep) {
            record.add_attribute(kv.key, kv.value.into_anyvalue());
        }

        self.emit(record);
    }
}

trait IntoAnyValue {
    fn into_anyvalue(self) -> AnyValue;
}

impl IntoAnyValue for Value {
    fn into_anyvalue(self) -> AnyValue {
        match self {
            Self::Bool(b) => b.into(),
            Self::I64(i) => i.into(),
            Self::F64(f) => f.into(),
            Self::String(s) => s.into(),
            Self::Array(a) => a.into_anyvalue(),
            _ => unreachable!(),
        }
    }
}

impl IntoAnyValue for Array {
    fn into_anyvalue(self) -> AnyValue {
        match self {
            Self::Bool(items) => items.into_anyvalue(),
            Self::I64(items) => items.into_anyvalue(),
            Self::F64(items) => items.into_anyvalue(),
            Self::String(items) => items.into_anyvalue(),
            _ => unreachable!(),
        }
    }
}

impl<T: Into<AnyValue>> IntoAnyValue for Vec<T> {
    fn into_anyvalue(self) -> AnyValue {
        AnyValue::ListAny(Box::new(self.into_iter().map(|t| t.into()).collect()))
    }
}
