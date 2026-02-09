use std::time::SystemTime;

use opentelemetry::KeyValue;
use opentelemetry_semantic_conventions::attribute;
use rootcause::{
    Report, ReportMut, ReportRef,
    markers::{Dynamic, Local, ReportOwnershipMarker, Uncloneable},
};

use crate::attachments::AttachmentsExt;

pub const EXCEPTION: &'static str = "exception";

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

pub fn attributes_brief(rep: ReportRef<'_, Dynamic, Uncloneable, Local>) -> Vec<KeyValue> {
    let rep = rep.as_report_ref();
    vec![
        KeyValue::new(attribute::EXCEPTION_TYPE, rep.current_context_type_name()),
        KeyValue::new(
            attribute::EXCEPTION_MESSAGE,
            rep.format_current_context().to_string(),
        ),
    ]
}

pub fn attributes(rep: ReportRef<'_, Dynamic, Uncloneable, Local>) -> Vec<KeyValue> {
    let rep = rep.as_report_ref();
    vec![
        KeyValue::new(attribute::EXCEPTION_TYPE, rep.current_context_type_name()),
        KeyValue::new(
            attribute::EXCEPTION_MESSAGE,
            rep.format_current_context().to_string(),
        ),
        KeyValue::new(attribute::EXCEPTION_STACKTRACE, rep.to_string()),
    ]
}

pub fn timestamp(rep: ReportRef<'_, Dynamic, Uncloneable, Local>) -> SystemTime {
    rep.find_attachment_inner()
        .cloned()
        .unwrap_or_else(SystemTime::now)
}
