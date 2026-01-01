use core::fmt;
use std::{
    fmt::{Debug, write},
    marker::PhantomData,
    time::SystemTime,
};

use opentelemetry::{Context, SpanId, trace::TraceContextExt};
use rootcause::{
    Report, ReportMut,
    handlers::{self, AttachmentHandler},
    hooks::report_creation::AttachmentCollector,
    markers::{Local, Mutable, ObjectMarkerFor, SendSync},
    report_attachment::ReportAttachment,
};

trait ReportExts: Sized {
    fn attach_marker<A: 'static + Debug + Send + Sync>(self, attachment: A) -> Self;

    fn mark_as_if_sent(self) -> Self {
        self.attach_marker(SentTo(SpanId::INVALID))
    }

    fn mark_sent_to(self, span: SpanId) -> Self {
        self.attach_marker(SentTo(span))
    }

    fn mark_as_sent(self) -> Self {
        self.mark_sent_to(Context::current().span().span_context().span_id())
    }
}

impl<C: 'static> ReportExts for Report<C, Mutable, SendSync> {
    fn attach_marker<A: 'static + Debug + Send + Sync>(self, attachment: A) -> Self {
        self.attach_custom::<Marker<A>, A>(attachment)
    }
}

impl<C: 'static> ReportExts for Report<C, Mutable, Local> {
    fn attach_marker<A: 'static + Debug + Send + Sync>(self, attachment: A) -> Self {
        self.attach_custom::<Marker<A>, A>(attachment)
    }
}

impl<'a, C: 'static> ReportExts for ReportMut<'a, C, Local> {
    fn attach_marker<A: 'static + Debug + Send + Sync>(mut self, attachment: A) -> Self {
        self.attachments_mut()
            .push(ReportAttachment::new_custom::<Marker<A>>(attachment).into_dynamic());
        self
    }
}

impl<'a, C: 'static> ReportExts for ReportMut<'a, C, SendSync> {
    fn attach_marker<A: 'static + Debug + Send + Sync>(mut self, attachment: A) -> Self {
        self.attachments_mut()
            .push(ReportAttachment::new_custom::<Marker<A>>(attachment).into_dynamic());
        self
    }
}

impl<T, RE: ReportExts> ReportExts for Result<T, RE> {
    fn attach_marker<A: 'static + Debug + Send + Sync>(self, attachment: A) -> Self {
        self.map_err(|e| e.attach_marker(attachment))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Marker<T: 'static>(PhantomData<&'static T>);
impl<T: 'static + Debug + Send + Sync> AttachmentHandler<T> for Marker<T> {
    fn display(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Debug::fmt(value, formatter)
    }

    fn debug(value: &T, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Debug::fmt(&value, formatter)
    }

    fn preferred_formatting_style(
        _value: &T,
        _report_formatting_function: handlers::FormattingFunction,
    ) -> handlers::AttachmentFormattingStyle {
        handlers::AttachmentFormattingStyle {
            placement: handlers::AttachmentFormattingPlacement::Hidden,
            function: handlers::FormattingFunction::Debug,
            priority: i32::MIN,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SentTo(pub SpanId);

pub struct ReportTimestamps;
impl AttachmentCollector<SystemTime> for ReportTimestamps {
    type Handler = Marker<SystemTime>;

    fn collect(&self) -> SystemTime {
        SystemTime::now()
    }
}
