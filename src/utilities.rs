use std::time::SystemTime;

use opentelemetry::KeyValue;
use opentelemetry_semantic_conventions::attribute;
use rootcause::{
    Report, ReportMut, ReportRef,
    markers::{Dynamic, Local, ReportOwnershipMarker, Uncloneable},
    report_attachment::ReportAttachmentRef,
    report_attachments::ReportAttachments,
};

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

    fn find_attachment_inner<A: 'static>(&self) -> Option<&A> {
        self.find_attachment::<A>().map(|a| a.inner())
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
