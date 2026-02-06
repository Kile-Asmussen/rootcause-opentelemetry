use std::{borrow::Cow, time::SystemTime};

use opentelemetry::{
    Context, KeyValue,
    trace::{Span, SpanRef, TraceContextExt, Tracer},
};
use opentelemetry_semantic_conventions::attribute;
use rootcause::{
    Report, ReportMut, ReportRef,
    hooks::builtin_hooks::location::Location,
    markers::{Dynamic, Local, Mutable, ReportOwnershipMarker, Uncloneable},
};
use rootcause_backtrace::Backtrace;

use crate::attachments::AttachmentsExt;

pub trait AsReportRef {
    fn as_report_ref(&self) -> ReportRef<'_, Dynamic, Uncloneable, Local>;
}

impl<C: ?Sized, O: ReportOwnershipMarker, T> AsReportRef for Report<C, O, T> {
    fn as_report_ref(&self) -> ReportRef<'_, Dynamic, Uncloneable, Local> {
        self.as_ref().into_dynamic().into_uncloneable().into_local()
    }
}

impl<'a, C: ?Sized, O, T> AsReportRef for ReportRef<'a, C, O, T> {
    fn as_report_ref(&self) -> ReportRef<'_, Dynamic, Uncloneable, Local> {
        self.into_dynamic().into_uncloneable().into_local()
    }
}

impl<'a, C: ?Sized, T> AsReportRef for ReportMut<'a, C, T> {
    fn as_report_ref(&self) -> ReportRef<'_, Dynamic, Uncloneable, Local> {
        self.as_ref().into_dynamic().into_local()
    }
}

pub trait SpanRefExt: Sized {
    fn record_error_report_event(self, rep: &impl AsReportRef);
}

impl<'a> SpanRefExt for SpanRef<'a> {
    fn record_error_report_event(self, rep: &impl AsReportRef) {
        let rep = rep.as_report_ref();
        self.add_event_with_timestamp(
            "exception",
            rep.find_attachment_inner::<SystemTime>()
                .cloned()
                .unwrap_or_else(|| SystemTime::now()),
            vec![
                KeyValue::new(attribute::EXCEPTION_TYPE, rep.current_context_type_name()),
                KeyValue::new(
                    attribute::EXCEPTION_MESSAGE,
                    rep.format_current_context().to_string(),
                ),
                KeyValue::new(attribute::EXCEPTION_STACKTRACE, rep.to_string()),
            ],
        );
    }
}

pub trait SpanExt {
    fn record_error_report_event(&mut self, rep: &impl AsReportRef);
}

impl<'a, S: Span> SpanExt for S {
    fn record_error_report_event(&mut self, rep: &impl AsReportRef) {
        let rep = rep.as_report_ref();
        self.add_event_with_timestamp(
            "exception",
            rep.find_attachment_inner::<SystemTime>()
                .cloned()
                .unwrap_or_else(|| SystemTime::now()),
            vec![
                KeyValue::new(attribute::EXCEPTION_TYPE, rep.current_context_type_name()),
                KeyValue::new(
                    attribute::EXCEPTION_MESSAGE,
                    rep.format_current_context().to_string(),
                ),
                KeyValue::new(attribute::EXCEPTION_STACKTRACE, rep.to_string()),
            ],
        );
    }
}
