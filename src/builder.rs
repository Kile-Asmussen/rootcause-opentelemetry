
use opentelemetry::{Context, StringValue, trace::{Span, Status, TraceContextExt}};
use rootcause::{Report, ReportRef, markers::{ObjectMarkerFor, ReportOwnershipMarker}};
use std::{any::TypeId, time::SystemTime};

use crate::{ExceptionEventSpec, span::SpanTraitExt, span::SpanRefExt, spec::ExceptionEventConfig};

pub trait ExceptionEventBuilder: Sized {
    type Context: 'static;
    type ObjectMarker: 'static + ReportOwnershipMarker;
    type ThreadSafety: 'static;

    type WrappedReport;
    
    fn send(self) -> Self::WrappedReport;
    fn send_to(self, span: &mut impl Span) -> Self::WrappedReport;

    fn underlying_report_ref(&self)
        -> Option<ReportRef<'_, Self::Context,
            <Self::ObjectMarker as ReportOwnershipMarker>::RefMarker,
            Self::ThreadSafety>>;

    fn spec(&mut self) -> Option<&mut ExceptionEventSpec>;

    fn status(self, status: Status) -> Self;
}

pub struct ReportWrapper<C: 'static, O: 'static, T: 'static> {
    report: Report<C, O, T>,
    status: Status,
    spec: ExceptionEventSpec
}

impl<C: 'static, O: 'static, T: 'static> ReportWrapper<C, O, T> {
    pub(crate) fn new(report: Report<C, O, T>) -> Self {
        Self {
            report,
            status: Status::Unset,
            spec: ExceptionEventSpec::default()
        }
    }
}

impl<C, O, T> ExceptionEventBuilder for ReportWrapper<C, O, T>
    where
        C: 'static,
        O: 'static + ReportOwnershipMarker,
        T: 'static,
{
    type Context = C;

    type ObjectMarker = O;

    type ThreadSafety = T;

    type WrappedReport = Report<C, O, T>;

    fn send(self) -> Self::WrappedReport {
        let Self { report, status, spec } = self;
        let ctx = Context::current();
        let span = ctx.span();
        span.add_exception_event(report.as_ref(), spec);
        if status != Status::Unset {
            span.set_status(status);
        }
        report
    }

    fn send_to(self, span: &mut impl Span) -> Self::WrappedReport {
        let Self { report, status, spec } = self;
        span.add_exception_event(report.as_ref(), spec);
        if status != Status::Unset {
            span.set_status(status);
        }
        report
    }

    fn underlying_report_ref(&self)
        -> Option<ReportRef<'_, Self::Context,
            <Self::ObjectMarker as ReportOwnershipMarker>::RefMarker,
            Self::ThreadSafety>>
    {
        Some(self.report.as_ref())
    }

    fn spec(&mut self) -> Option<&mut ExceptionEventSpec> {
        Some(&mut self.spec)
    }

    fn status(mut self, status: Status) -> Self {
        self.status = status; self
    }
}

impl<X, C, O, T> ExceptionEventBuilder for Result<X, ReportWrapper<C, O, T>>
    where
        C: 'static,
        O: 'static + ReportOwnershipMarker,
        T: 'static,
{
    type Context = C;
    type ObjectMarker = O;
    type ThreadSafety = T;

    type WrappedReport = Result<X, Report<C, O, T>>;

    fn send(self) -> Self::WrappedReport {
        match self {
            Self::Ok(x) => Ok(x),
            Self::Err(r) => Err(r.send()),
        }
    }
        
    fn send_to(self, span: &mut impl Span) -> Self::WrappedReport {
        match self {
            Self::Ok(x) => Ok(x),
            Self::Err(r) => Err(r.send_to(span)),
        }
    }

    fn underlying_report_ref(&self)
        -> Option<ReportRef<'_, Self::Context,
            <Self::ObjectMarker as ReportOwnershipMarker>::RefMarker,
            Self::ThreadSafety>>
    {
        match self {
            Self::Ok(_) => None,
            Self::Err(r) => r.underlying_report_ref(),
        }
    }

    fn spec(&mut self) -> Option<&mut ExceptionEventSpec> {
        match self {
            Self::Ok(_) => todo!(),
            Self::Err(r) => r.spec(),
        }
    }

    fn status(self, status: Status) -> Self {
        self.map_err(|r| r.status(status))
    }
}

macro_rules! impl_eventconfig_for_builder {
    () => {
        fn ex_type(mut self) -> Self {
            self.spec().map(|s| s.ex_type()); self
        }

        fn set_ex_type(mut self, ex_type: impl Into<StringValue>) -> Self {
            self.spec().map(|s| s.set_ex_type(ex_type)); self
        }

        fn custom_message(mut self, msg: impl Into<StringValue>) -> Self {
            self.spec().map(|s| s.custom_message(msg)); self
        }

        fn timestamped(mut self) -> Self {
            self.spec().map(|s| s.timestamped()); self
        }

        fn set_timestamp(mut self, systime: SystemTime) -> Self {
            self.spec().map(|s| s.set_timestamp(systime)); self
        }

        fn timestamp_now(mut self) -> Self {
            self.spec().map(|s| s.timestamp_now()); self
        }

        fn backtrace(mut self) -> Self {
            self.spec().map(|s| s.backtrace()); self
        }

        fn override_backtrace(mut self, backtrace: String) -> Self {
            self.spec().map(|s| s.override_backtrace(backtrace)); self
        }

        fn escaped(mut self, has_escaped: bool) -> Self {
            self.spec().map(|s| s.escaped(has_escaped)); self
        }

        fn add_attacment(mut self, at: impl Into<StringValue>) -> Self {
            self.spec().map(|s| s.add_attacment(at)); self
        }

        fn all_attachments(mut self) -> Self {
            self.spec().map(|s| s.all_attachments()); self
        }

        fn attachments(mut self) -> Self {
            self.spec().map(|s| s.attachments()); self
        }

        fn attachments_of_type_id(mut self, type_id: TypeId) -> Self {
            self.spec().map(|s| s.attachments_of_type_id(type_id)); self
        }

        fn recurse(mut self) -> Self {
            self.spec().map(|s| s.recurse()); self
        }

        fn children(mut self, actions: ExceptionEventSpec) -> Self {
            self.spec().map(|s| s.children(actions)); self
        }
    };
}

impl<C, O, T> ExceptionEventConfig for ReportWrapper<C, O, T>
    where
        C: 'static,
        O: 'static + ReportOwnershipMarker,
        T: 'static,
{
    impl_eventconfig_for_builder!();
}

impl<X, C, O, T> ExceptionEventConfig for Result<X, ReportWrapper<C, O, T>>
    where
        C: 'static,
        O: 'static + ReportOwnershipMarker,
        T: 'static,
{
    impl_eventconfig_for_builder!();
}