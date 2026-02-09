use std::time::SystemTime;

use opentelemetry::{
    KeyValue,
    trace::{Span, SpanContext, SpanRef, Status, noop::NoopSpan},
};
use opentelemetry_semantic_conventions::attribute;
use rootcause::{
    ReportRef,
    markers::{Dynamic, Local, Uncloneable},
};

use crate::utilities::{
    AsReportRef, AttachmentsExt, EXCEPTION, attributes, attributes_brief, timestamp,
};

/// Extension trait for the [`SpanRef<'_>`] type
/// which is returned by [`Context::span`](opentelemetry::context::Context::span).
pub trait SpanRefReportExt: Sized {
    /// Returns a builder-pattern for turning reports into events on a span.
    ///
    /// See [`RecordErrorReport`]
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

/// Extension trait for types implementing [`Span`].
pub trait SpanReportExt: Span + Sized {
    #[must_use]
    fn record_error_report<'b>(
        &'b mut self,
        rep: &'b impl AsReportRef,
    ) -> RecordErrorReport<'b, Self>;
}

impl<S: Span> SpanReportExt for S {
    /// Returns a builder-pattern for turning reports into events on a span.
    ///
    /// ```rust
    /// todo!()
    /// ```
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

/// Builder for configuring how [`Report`](rootcause::Report)s are recorded on a span.
///
/// It contains either a [`SpanRef`] or some
/// concrete implementation of the [`Span`] trait, because OTel is a little janky.
#[must_use]
pub struct RecordErrorReport<'a, S: Span> {
    spanish: SpanIsh<'a, S>,
    report: ReportRef<'a, Dynamic, Uncloneable, Local>,
}

impl<'a, S: Span> RecordErrorReport<'a, S> {
    /// Record the [`Report`](rootcause::Report) as an `exception` event on the span.
    ///
    /// ## Attributes & Details
    /// - The timestamp of the event is given by a [`SystemTime`](std::time::SystemTime)-typed attachment, or defaults to [`now()`](std::time::SystemTime::now) if not found.
    /// - `exception.type` is [`.current_context_type_name()`](rootcause::Report::current_context_type_name).
    /// - `exception.message` is [`.format_current_context().to_string()`](rootcause::Report::format_current_context).
    /// - `exception.stacktrace` is just `.to_string()` of the [`Report`](rootcause::Report) itself
    ///
    /// [`SystemTime`](std::time::SystemTime) attachments are
    /// provided report creation hook [`OpenTelemetryMetadataCollector`](crate::attachments::OpenTelemetryMetadataCollector).
    ///
    /// ## Spec   
    /// [Semantic conventions for exceptions on spans](https://opentelemetry.io/docs/specs/semconv/exceptions/exceptions-spans/)
    pub fn as_event(mut self) -> Self {
        self.spanish.add_event_with_timestamp(
            EXCEPTION,
            timestamp(self.report),
            attributes(self.report),
        );
        self
    }

    /// Record the [`Report`] as an `exception` event on the span, as in [`Self::as_event`],
    /// but omit the optional `exception.stacktrace` attribute for brevity.
    pub fn as_event_brief(mut self) -> Self {
        self.spanish.add_event_with_timestamp(
            EXCEPTION,
            timestamp(self.report),
            attributes_brief(self.report),
        );
        self
    }

    /// Set the span status to [`Error`](Status::Error).
    ///
    /// ## Attributes & Details
    /// - `description` of the status itself is [`.format_current_context().to_string()`](rootcause::Report::format_current_context)
    /// - `error.type` attribute is [`.current_context_type_name()`](rootcause::Report::current_context_type_name).
    ///
    /// ## Spec
    /// [Recording errors > Recording errors on spans](https://opentelemetry.io/docs/specs/semconv/general/recording-errors/#recording-errors-on-spans)
    pub fn with_error_status(mut self) -> Self {
        self.spanish.set_attributes([KeyValue::new(
            attribute::ERROR_TYPE,
            self.report.current_context_type_name(),
        )]);
        self.spanish.set_status(Status::Error {
            description: self.report.format_current_context().to_string().into(),
        });
        self
    }

    /// End the span.
    ///
    /// ## Attributes & Details
    /// - The timestamp of the event is given by a [`SystemTime`](std::time::SystemTime)-typed attachment, or defaults to [`now()`](std::time::SystemTime::now) if not found.
    ///
    /// [`SystemTime`](std::time::SystemTime) attachments are
    /// provided report creation hook [`OpenTelemetryMetadataCollector`](crate::attachments::OpenTelemetryMetadataCollector).
    pub fn end_span(mut self) -> Self {
        self.spanish.end_with_timestamp(timestamp(self.report));
        self
    }

    /// ⚠️ This is speculative functionality
    ///
    /// Record the exception-related attributes on the span itself
    /// instead of as an event on the span.
    ///
    /// ## Attributes & Details
    /// - `exception.type` is [`.current_context_type_name()`](rootcause::Report::current_context_type_name).
    /// - `exception.message` is [`.format_current_context().to_string()`](rootcause::Report::format_current_context).
    /// - `exception.stacktrace` is just `.to_string()` of the [`Report`](rootcause::Report) itself
    ///
    /// ## Spec
    ///
    /// Discussion: [`opentelemetry-specification/#4429`](https://github.com/open-telemetry/opentelemetry-specification/issues/4429)
    ///
    /// Attributes taken from: [Semantic conventions for exceptions on spans](https://opentelemetry.io/docs/specs/semconv/exceptions/exceptions-spans/)
    pub fn on_span_attributes(mut self) -> Self {
        self.spanish.set_attributes(attributes(self.report));
        self
    }

    /// ⚠️ This is speculative functionality
    ///
    /// Record the exception-related attributes on the span itself
    /// as in [`Self::on_span_attributes`], but omit the `exception.stacktrace`
    /// attribute for brevity.
    pub fn as_span_attributes_brief(mut self) -> Self {
        self.spanish.set_attributes(attributes_brief(self.report));
        self
    }

    /// Traverse the report and all child reports looking for attachments
    /// of type [`SpanContext`], adding appropriate span links on the current
    /// span to indicate causality.
    ///
    /// ## Attributes & Details
    /// - The linked spans' tracing contexts are taken from [`SpanContext`]-typed attachments on the reports. Reports without such attachments are not linked, and reports originating in the current span are not linked either.
    /// - `exception.type` is [`.current_context_type_name()`](rootcause::Report::current_context_type_name).
    /// - `exception.message` is [`.format_current_context().to_string()`](rootcause::Report::format_current_context).
    /// - `exception.stacktrace` is omitted for brevity.
    ///
    /// [`SpanContext`] attachments are
    /// provided report creation hook [`OpenTelemetryMetadataCollector`](crate::attachments::OpenTelemetryMetadataCollector).
    ///
    /// ## Spec
    /// [Traces > Span Links](https://opentelemetry.io/docs/concepts/signals/traces/#span-links)
    ///
    /// Attributes taken from: [Semantic conventions for exceptions on spans](https://opentelemetry.io/docs/specs/semconv/exceptions/exceptions-spans/)
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

    /// Traverse the report and all child reports to create span links, but
    /// only set the `error.type` attribute for brevity.
    ///
    /// ## Attributes & Details
    /// - `error.type` attribute is [`.current_context_type_name()`](rootcause::Report::current_context_type_name).
    ///
    /// ## Spec
    /// [Traces > Span Links](https://opentelemetry.io/docs/concepts/signals/traces/#span-links)
    ///
    /// Attributes taken from: [Recording errors > Recording errors on spans](https://opentelemetry.io/docs/specs/semconv/general/recording-errors/#recording-errors-on-spans)
    pub fn link_child_report_spans_brief(mut self) -> Self {
        let curr_ctx = self.spanish.span_context().clone();

        for sub_rep in self.report.iter_reports() {
            if let Some(ctx) = sub_rep.find_attachment_inner::<SpanContext>()
                && ctx != &curr_ctx
            {
                self.spanish.add_link(
                    ctx.clone(),
                    [KeyValue::new(
                        attribute::ERROR_TYPE,
                        sub_rep.current_context_type_name(),
                    )],
                );
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
    fn set_attributes(&mut self, attributes: impl IntoIterator<Item = KeyValue>) {
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

    fn add_link(
        &mut self,
        span_context: SpanContext,
        attributes: impl IntoIterator<Item = KeyValue>,
    ) {
        match self {
            Self::SpanRef(span) => span.add_link(span_context, attributes.into_iter().collect()),
            Self::MutSpan(span) => span.add_link(span_context, attributes.into_iter().collect()),
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
