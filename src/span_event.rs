use std::time::SystemTime;

use opentelemetry::{
    KeyValue,
    trace::{Span, SpanContext, SpanRef, Status, noop::NoopSpan},
};
use rootcause::{
    ReportRef,
    markers::{Dynamic, Local, Uncloneable},
};

use crate::utilities::{
    AsReportRef, AttachmentsExt, EXCEPTION, attributes, attributes_brief, timestamp,
};

pub trait SpanRefReportExt: Sized {
    #[must_use]
    fn record_error_report<'b>(
        &'b self,
        rep: &'b impl AsReportRef,
    ) -> RecordErrorReport<'b, NoopSpan>;
}

impl<'a> SpanRefReportExt for SpanRef<'a> {
    fn record_error_report<'b>(
        &'b self,
        rep: &'b impl AsReportRef,
    ) -> RecordErrorReport<'b, NoopSpan> {
        RecordErrorReport {
            spanish: SpanIsh::SpanRef(self),
            report: rep.as_report_ref(),
        }
    }
}

pub trait SpanReportExt: Span + Sized {
    #[must_use]
    fn record_error_report<'b>(
        &'b mut self,
        rep: &'b impl AsReportRef,
    ) -> RecordErrorReport<'b, Self>;
}

impl<S: Span> SpanReportExt for S {
    fn record_error_report<'b>(
        &'b mut self,
        rep: &'b impl AsReportRef,
    ) -> RecordErrorReport<'b, Self> {
        RecordErrorReport {
            spanish: SpanIsh::MutSpan(self),
            report: rep.as_report_ref(),
        }
    }
}

#[must_use]
pub struct RecordErrorReport<'a, S: Span> {
    spanish: SpanIsh<'a, S>,
    report: ReportRef<'a, Dynamic, Uncloneable, Local>,
}

impl<'a, S: Span> RecordErrorReport<'a, S> {
    pub fn as_event(mut self) -> Self {
        self.spanish.add_event_with_timestamp(
            EXCEPTION,
            timestamp(self.report),
            attributes(self.report),
        );
        self
    }

    pub fn as_event_brief(mut self) -> Self {
        self.spanish.add_event_with_timestamp(
            EXCEPTION,
            timestamp(self.report),
            attributes_brief(self.report),
        );
        self
    }

    pub fn with_error_status(mut self) -> Self {
        self.spanish.set_status(Status::Error {
            description: EXCEPTION.into(),
        });
        self
    }

    pub fn end_span(mut self) -> Self {
        self.spanish.end_with_timestamp(timestamp(self.report));
        self
    }

    pub fn on_span_attributes(mut self) -> Self {
        self.spanish.set_attributes(attributes(self.report));
        self
    }

    pub fn as_span_attributes_brief(mut self) -> Self {
        self.spanish.set_attributes(attributes_brief(self.report));
        self
    }

    pub fn link_child_report_spans(mut self) -> Self {
        let curr_ctx = self.spanish.span_context().clone();

        for sub_rep in self.report.iter_reports() {
            if let Some(ctx) = sub_rep.find_attachment_inner::<SpanContext>()
                && ctx != &curr_ctx
            {
                self.spanish
                    .add_link(ctx.clone(), attributes_brief(sub_rep));
            }
        }

        self
    }
}

enum SpanIsh<'a, S: Span> {
    SpanRef(&'a SpanRef<'a>),
    MutSpan(&'a mut S),
}

impl<'a, S: Span> SpanIsh<'a, S> {
    fn set_attributes(&mut self, attributes: Vec<KeyValue>) {
        match self {
            Self::SpanRef(span) => span.set_attributes(attributes),
            Self::MutSpan(span) => span.set_attributes(attributes),
        };
    }

    fn set_status(&mut self, status: Status) {
        match self {
            Self::SpanRef(span) => span.set_status(status),
            Self::MutSpan(span) => span.set_status(status),
        }
    }

    fn add_link(&mut self, span_context: SpanContext, attributes: Vec<KeyValue>) {
        match self {
            Self::SpanRef(span) => span.add_link(span_context, attributes),
            Self::MutSpan(span) => span.add_link(span_context, attributes),
        }
    }

    fn add_event_with_timestamp(
        &mut self,
        name: &'static str,
        timestamp: SystemTime,
        attributes: Vec<KeyValue>,
    ) {
        match self {
            Self::SpanRef(span) => span.add_event_with_timestamp(name, timestamp, attributes),
            Self::MutSpan(span) => span.add_event_with_timestamp(name, timestamp, attributes),
        }
    }

    fn span_context(&self) -> &SpanContext {
        match self {
            Self::SpanRef(span) => span.span_context(),
            Self::MutSpan(span) => span.span_context(),
        }
    }

    fn end_with_timestamp(&mut self, timestamp: SystemTime) {
        match self {
            Self::SpanRef(span) => span.end_with_timestamp(timestamp),
            Self::MutSpan(span) => span.end_with_timestamp(timestamp),
        }
    }
}
