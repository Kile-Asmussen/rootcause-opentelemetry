
use opentelemetry::{StringValue, trace::Status};
use rootcause::{Report, ReportRef, markers::{ObjectMarkerFor, ReportOwnershipMarker}};
use std::{any::TypeId, time::SystemTime};

use crate::{OTelEventSpec, spec::OTelEventConfig};

pub trait OTelEventBuilder: Sized {
    type Context: 'static;
    type ObjectMarker: 'static + ObjectMarkerFor<Self::Context> + ReportOwnershipMarker;
    type ThreadSafety: 'static;

    type WrappedReport;
    
    fn send(self) -> Self::WrappedReport;

    fn underlying_report_ref<'a>(&'a self)
        -> Option<ReportRef<'a, Self::Context,
            <Self::ObjectMarker as ReportOwnershipMarker>::RefMarker,
            Self::ThreadSafety>>;

    fn spec(&mut self) -> Option<&mut OTelEventSpec>;

    fn status(self, status: Status) -> Self;
}

pub struct OTelReportWrapper<C: 'static, O: 'static, T: 'static> {
    report: Report<C, O, T>,
    status: Status,
    spec: OTelEventSpec
}

impl<C: 'static, O: 'static, T: 'static> OTelReportWrapper<C, O, T> {
    pub(crate) fn new(report: Report<C, O, T>) -> Self {
        Self {
            report,
            status: Status::Unset,
            spec: OTelEventSpec::default()
        }
    }
}

impl<C, O, T> OTelEventBuilder for OTelReportWrapper<C, O, T>
    where
        C: 'static,
        O: 'static + ObjectMarkerFor<C> + ReportOwnershipMarker,
        T: 'static,
{
    type Context = C;

    type ObjectMarker = O;

    type ThreadSafety = T;

    type WrappedReport = Report<C, O, T>;

    fn send(self) -> Self::WrappedReport {
        self.report
    }

    fn underlying_report_ref<'a>(&'a self)
        -> Option<ReportRef<'a, Self::Context,
            <Self::ObjectMarker as ReportOwnershipMarker>::RefMarker,
            Self::ThreadSafety>>
    {
        Some(self.report.as_ref())
    }

    fn spec(&mut self) -> Option<&mut OTelEventSpec> {
        Some(&mut self.spec)
    }

    fn status(mut self, status: Status) -> Self {
        self.status = status; self
    }
}

impl<X, C, O, T> OTelEventBuilder for Result<X, OTelReportWrapper<C, O, T>>
    where
        C: 'static,
        O: 'static + ObjectMarkerFor<C> + ReportOwnershipMarker,
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

    fn underlying_report_ref<'a>(&'a self)
        -> Option<ReportRef<'a, Self::Context,
            <Self::ObjectMarker as ReportOwnershipMarker>::RefMarker,
            Self::ThreadSafety>>
    {
        match self {
            Self::Ok(_) => None,
            Self::Err(r) => r.underlying_report_ref(),
        }
    }

    fn spec(&mut self) -> Option<&mut OTelEventSpec> {
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

        fn children(mut self, actions: OTelEventSpec) -> Self {
            self.spec().map(|s| s.children(actions)); self
        }
    };
}

impl<C, O, T> OTelEventConfig for OTelReportWrapper<C, O, T>
    where
        C: 'static,
        O: 'static + ObjectMarkerFor<C> + ReportOwnershipMarker,
        T: 'static,
{
    impl_eventconfig_for_builder!();
}

impl<X, C, O, T> OTelEventConfig for Result<X, OTelReportWrapper<C, O, T>>
    where
        C: 'static,
        O: 'static + ObjectMarkerFor<C> + ReportOwnershipMarker,
        T: 'static,
{
    impl_eventconfig_for_builder!();
}