use std::time::SystemTime;

use rootcause::{Report, handlers, hooks::report_creation::AttachmentCollector, markers::{ObjectMarkerFor, ReportOwnershipMarker}};

use crate::{builder::{OTelEventBuilder, OTelReportWrapper}, spec::{OTelEventConfig, OTelEventSpec}};

pub struct SystemTimeCollector;
impl AttachmentCollector<SystemTime> for SystemTimeCollector {
    type Handler = handlers::Debug;

    fn collect(&self) -> SystemTime {
        SystemTime::now()
    }
}

trait ReportExt: Sized {
    type Builder: OTelEventBuilder + OTelEventConfig;

    fn otel(self) -> Self::Builder {
        self.otel_raw()
            .ex_type()
            .timestamped()
            .backtrace()
            .attachments()
    }

    fn otel_raw(self) -> Self::Builder;
}

impl<C, O, T> ReportExt for Report<C, O, T>
    where
        C: 'static,
        O: 'static + ObjectMarkerFor<C> + ReportOwnershipMarker,
        T: 'static,
{
    type Builder = OTelReportWrapper<C, O, T>;
        
    fn otel_raw(self) -> Self::Builder {
        OTelReportWrapper::new(self)
    }
}

impl<X, C, O, T> ReportExt for Result<X, Report<C, O, T>>
    where
        C: 'static,
        O: 'static + ObjectMarkerFor<C> + ReportOwnershipMarker,
        T: 'static,
{
    type Builder = Result<X, OTelReportWrapper<C, O, T>>;
        
    fn otel_raw(self) -> Self::Builder {
        self.map_err(OTelReportWrapper::new)
    }
}