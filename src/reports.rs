use std::time::SystemTime;

use rootcause::{Report, handlers::{self, AttachmentHandler, ContextHandler}, hooks::report_creation::AttachmentCollector, markers::{ObjectMarkerFor, ReportOwnershipMarker}};
use serde::{Deserialize, Serialize};

use crate::{builder::{ExceptionEventBuilder, ReportWrapper}, spec::{ExceptionEventConfig, ExceptionEventConfigExt, ExceptionEventSpec}};

pub struct SystemTimeCollector;
impl AttachmentCollector<SystemTime> for SystemTimeCollector {
    type Handler = handlers::Debug;

    fn collect(&self) -> SystemTime {
        SystemTime::now()
    }
}

pub struct JsonHandler;

impl<A : Serialize> AttachmentHandler<A> for JsonHandler {
    fn display(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match serde_json::ser::to_string_pretty(value) {
            Ok(s) => formatter.write_str(&s),
            Err(e) => Err(std::fmt::Error),
        }
    }

    fn debug(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match serde_json::ser::to_string(value) {
            Ok(s) => formatter.write_str(&s),
            Err(e) => Err(std::fmt::Error),
        }
    }
}

impl<C: Serialize> ContextHandler<C> for JsonHandler {
    fn source(value: &C) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }

    fn display(value: &C, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <JsonHandler as AttachmentHandler<C>>::display(value, formatter)
    }

    fn debug(value: &C, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <JsonHandler as AttachmentHandler<C>>::debug(value, formatter)
    }
}

pub trait ReportExt: Sized {
    type Builder: ExceptionEventBuilder + ExceptionEventConfig;

    fn otel_raw(self) -> Self::Builder;

    fn otel(self) -> Self::Builder {
        self.otel_raw()
            .ex_type()
            .timestamped()
            .backtrace()
    }

    fn otel_spec(self, spec: ExceptionEventSpec) -> Self::Builder {
        self.otel_raw().config(spec)
    }
}

impl<C, O, T> ReportExt for Report<C, O, T>
    where
        C: 'static,
        O: 'static + ReportOwnershipMarker,
        T: 'static,
{
    type Builder = ReportWrapper<C, O, T>;
        
    fn otel_raw(self) -> Self::Builder {
        ReportWrapper::new(self)
    }
}

impl<X, C, O, T> ReportExt for Result<X, Report<C, O, T>>
    where
        C: 'static,
        O: 'static + ReportOwnershipMarker,
        T: 'static,
{
    type Builder = Result<X, ReportWrapper<C, O, T>>;
        
    fn otel_raw(self) -> Self::Builder {
        self.map_err(ReportWrapper::new)
    }
}