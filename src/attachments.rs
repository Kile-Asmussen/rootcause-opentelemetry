use core::fmt;
use std::{
    fmt::{Debug, write},
    time::SystemTime,
};

use opentelemetry::SpanId;
use rootcause::{
    Report, ReportMut,
    handlers::{self, AttachmentHandler},
    hooks::report_creation::AttachmentCollector,
    markers::{Local, Mutable, ObjectMarkerFor, SendSync},
    report_attachment::ReportAttachment,
};

use crate::attachments;

trait ReportExts {
    fn attach_marker<A: 'static + Debug + Send + Sync>(self, attachment: A) -> Self;
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

#[derive(Debug, Clone, Copy)]
struct Marker<T>(T);
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
pub struct SkipSend;

#[derive(Debug, Clone, Copy)]
pub struct SentTo(pub SpanId);

pub struct ReportTimestamping;
impl AttachmentCollector<SystemTime> for ReportTimestamping {
    type Handler = ReportTimestamping;

    fn collect(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl AttachmentHandler<SystemTime> for ReportTimestamping {
    fn display(value: &SystemTime, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }

    fn debug(value: &SystemTime, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }

    fn preferred_formatting_style(
        value: &SystemTime,
        report_formatting_function: handlers::FormattingFunction,
    ) -> handlers::AttachmentFormattingStyle {
        handlers::AttachmentFormattingStyle {
            placement: handlers::AttachmentFormattingPlacement::Hidden,
            function: report_formatting_function,
            priority: i32::MIN,
        }
    }
}
