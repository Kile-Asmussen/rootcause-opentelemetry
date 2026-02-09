use core::fmt;
use std::{
    fmt::{Debug, Write},
    i32,
    time::SystemTime,
};

use opentelemetry::{
    Context,
    trace::{SpanContext, TraceContextExt},
};
use rootcause::{
    ReportMut,
    handlers::{
        self, AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
        FormattingFunction,
    },
    hooks::{attachment_formatter::AttachmentFormatterHook, report_creation::ReportCreationHook},
    markers::{self, Local, SendSync},
    report_attachment::ReportAttachmentRef,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct OpenTelemetryMetadataCollector<const TIMESTAMPS: bool = true> {
    _priv: (),
}

impl OpenTelemetryMetadataCollector<true> {
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl OpenTelemetryMetadataCollector<false> {
    pub fn no_timestamps() -> Self {
        Self { _priv: () }
    }
}

impl<const TIMESTAMPS: bool> AttachmentHandler<SystemTime>
    for OpenTelemetryMetadataCollector<TIMESTAMPS>
{
    fn display(_value: &SystemTime, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }

    fn debug(_value: &SystemTime, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }

    fn preferred_formatting_style(
        _value: &SystemTime,
        report_formatting_function: handlers::FormattingFunction,
    ) -> handlers::AttachmentFormattingStyle {
        handlers::AttachmentFormattingStyle {
            placement: handlers::AttachmentFormattingPlacement::Hidden,
            function: report_formatting_function,
            priority: i32::MIN,
        }
    }
}

impl<const TIMESTAMPS: bool> AttachmentHandler<SpanContext>
    for OpenTelemetryMetadataCollector<TIMESTAMPS>
{
    fn display(value: &SpanContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "00-{:x}-{:x}-{:02x}",
            value.trace_id(),
            value.span_id(),
            value.trace_flags(),
        )?;
        let header = value.trace_state().header();
        if !header.is_empty() {
            formatter.write_char('\n')?;
            formatter.write_str(&header)?;
        }

        Ok(())
    }

    fn debug(value: &SpanContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(value, formatter)
    }

    fn preferred_formatting_style(
        value: &SpanContext,
        function: FormattingFunction,
    ) -> handlers::AttachmentFormattingStyle {
        let placement = if function == FormattingFunction::Debug {
            AttachmentFormattingPlacement::Inline
        } else if value.is_remote() {
            AttachmentFormattingPlacement::InlineWithHeader {
                header: "TRACE ↗"
            }
        } else {
            AttachmentFormattingPlacement::InlineWithHeader { header: "TRACE" }
        };

        AttachmentFormattingStyle {
            placement,
            function,
            priority: 5,
        }
    }
}

impl<const TIMESTAMPS: bool> ReportCreationHook for OpenTelemetryMetadataCollector<TIMESTAMPS> {
    fn on_local_creation(&self, mut report: ReportMut<'_, markers::Dynamic, Local>) {
        if TIMESTAMPS {
            report = report.attach_custom::<OpenTelemetryMetadataCollector, _>(SystemTime::now());
        }
        let ctx = Context::current();
        let span = ctx.span();
        let span_ctx = span.span_context();
        if span_ctx.is_valid() {
            let _ = report.attach_custom::<OpenTelemetryMetadataCollector, _>(span_ctx.clone());
        }
    }

    fn on_sendsync_creation(&self, mut report: ReportMut<'_, markers::Dynamic, SendSync>) {
        report = report.attach_custom::<OpenTelemetryMetadataCollector, _>(SystemTime::now());
        let ctx = Context::current();
        let span = ctx.span();
        let span_ctx = span.span_context();
        if span_ctx.is_valid() {
            let _ = report.attach_custom::<OpenTelemetryMetadataCollector, _>(span_ctx.clone());
        }
    }
}

pub struct HideTraceAttachments;
impl AttachmentFormatterHook<SpanContext> for HideTraceAttachments {
    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, markers::Dynamic>,
        report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Hidden,
            function: report_formatting_function,
            priority: i32::MIN,
        }
    }
}
