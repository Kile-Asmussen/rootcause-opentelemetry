use core::fmt;
use std::{
    fmt::{Debug, Write, write},
    marker::PhantomData,
    time::SystemTime,
};

use opentelemetry::{
    Context, SpanId,
    trace::{Span, SpanContext, TraceContextExt, Tracer},
};
use rootcause::{
    Report, ReportMut, ReportRef,
    handlers::{
        self, AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
        FormattingFunction,
    },
    hooks::report_creation::{AttachmentCollector, ReportCreationHook},
    markers::{Local, Mutable, ObjectMarkerFor, SendSync},
    report_attachment::{ReportAttachment, ReportAttachmentRef},
    report_attachments::ReportAttachments,
};

#[derive(Debug, Clone, Copy)]
pub struct Invisible;
impl<T: 'static> AttachmentHandler<T> for Invisible {
    fn display(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }

    fn debug(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }

    fn preferred_formatting_style(
        _value: &T,
        report_formatting_function: handlers::FormattingFunction,
    ) -> handlers::AttachmentFormattingStyle {
        handlers::AttachmentFormattingStyle {
            placement: handlers::AttachmentFormattingPlacement::Hidden,
            function: report_formatting_function,
            priority: i32::MIN,
        }
    }
}

pub struct OTelMetadataCollector;
impl ReportCreationHook for OTelMetadataCollector {
    fn on_local_creation(&self, mut report: ReportMut<'_, rootcause::markers::Dynamic, Local>) {
        report = report.attach_custom::<Invisible, _>(SystemTime::now());
        let ctx = Context::current();
        eprintln!("Context in local hook: {:?}", ctx);
        let span = ctx.span();
        let span_ctx = span.span_context();
        if span_ctx.is_valid() {
            report.attach_custom::<SpanContextHandler, _>(span_ctx.clone());
        }
    }

    fn on_sendsync_creation(
        &self,
        mut report: ReportMut<'_, rootcause::markers::Dynamic, SendSync>,
    ) {
        report = report.attach_custom::<Invisible, _>(SystemTime::now());
        let ctx = Context::current();
        eprintln!("Context in sendsync hook: {:?}", ctx);
        let span = ctx.span();
        let span_ctx = span.span_context();
        if span_ctx.is_valid() {
            report.attach_custom::<SpanContextHandler, _>(span_ctx.clone());
        }
    }
}

pub struct SpanContextHandler;

impl AttachmentHandler<SpanContext> for SpanContextHandler {
    fn display(value: &SpanContext, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

    fn debug(value: &SpanContext, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

pub trait AttachmentsExt {
    fn find_attachment<A: 'static>(&self) -> Option<ReportAttachmentRef<'_, A>>;
    fn find_attachment_inner<A: 'static>(&self) -> Option<&A> {
        self.find_attachment::<A>().map(|a| a.inner())
    }
}

impl<T: 'static> AttachmentsExt for ReportAttachments<T> {
    fn find_attachment<A: 'static>(&self) -> Option<ReportAttachmentRef<'_, A>> {
        self.iter().find_map(|a| a.downcast_attachment())
    }
}

impl<C: 'static + ?Sized, O: 'static, T: 'static> AttachmentsExt for Report<C, O, T> {
    fn find_attachment<A: 'static>(&self) -> Option<ReportAttachmentRef<'_, A>> {
        self.attachments().find_attachment::<A>()
    }
}

impl<'a, C: 'static + ?Sized, O: 'static, T: 'static> AttachmentsExt for ReportRef<'a, C, O, T> {
    fn find_attachment<A: 'static>(&self) -> Option<ReportAttachmentRef<'_, A>> {
        self.attachments().find_attachment::<A>()
    }
}

impl<'a, C: 'static + ?Sized, T: 'static> AttachmentsExt for ReportMut<'a, C, T> {
    fn find_attachment<A: 'static>(&self) -> Option<ReportAttachmentRef<'_, A>> {
        self.attachments().find_attachment::<A>()
    }
}
