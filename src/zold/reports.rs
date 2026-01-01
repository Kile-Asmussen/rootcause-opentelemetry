use std::{fmt::Pointer, i32, time::SystemTime};

use opentelemetry::{Key, KeyValue, Value};
use rootcause::{
    IntoReport, Report,
    handlers::{self, AttachmentHandler, ContextHandler, Debug},
    hooks::report_creation::{AttachmentCollector, ReportCreationHook},
    markers::{Local, Mutable, ObjectMarkerFor, ReportOwnershipMarker, SendSync},
    prelude::ResultExt,
};
use serde::{Deserialize, Serialize};

use crate::{
    builder::{ExceptionEventBuilder, ReportWrapper},
    spec::{ExceptionEventConfig, ExceptionEventConfigExt, ExceptionEventSpec},
};

pub struct JsonHandler;

impl<A: Serialize> AttachmentHandler<A> for JsonHandler {
    fn display(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        serde_json::to_string_pretty(value)
            .map_err(|_| core::fmt::Error)
            .and_then(|s| formatter.write_str(&s))
    }

    fn debug(value: &A, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as AttachmentHandler<A>>::display(value, formatter)
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

pub struct OTelAttribute;
impl AttachmentHandler<KeyValue> for OTelAttribute {
    fn display(value: &KeyValue, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "{}: {}",
            value.key.as_str(),
            value.value.as_str()
        )
    }

    fn debug(value: &KeyValue, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{:?}", value)
    }

    fn preferred_formatting_style(
        _value: &KeyValue,
        _report_formatting_function: handlers::FormattingFunction,
    ) -> handlers::AttachmentFormattingStyle {
        handlers::AttachmentFormattingStyle {
            placement: handlers::AttachmentFormattingPlacement::Appendix {
                appendix_name: "OTel Attributes",
            },
            function: handlers::FormattingFunction::Display,
            priority: 0,
        }
    }
}

pub trait ResultExtExt<V, E>: ResultExt<V, E> {
    fn attach_attribute(
        self,
        key: impl Into<Key>,
        value: impl Into<Value>,
    ) -> Result<V, Report<E::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>;
}

impl<V, E, RE: ResultExt<V, E>> ResultExtExt<V, E> for RE {
    fn attach_attribute(
        self,
        key: impl Into<Key>,
        value: impl Into<Value>,
    ) -> Result<V, Report<<E>::Context, Mutable, SendSync>>
    where
        E: IntoReport<SendSync, Ownership = Mutable>,
    {
        self.attach_custom::<OTelAttribute, _>(KeyValue::new(key, value))
    }
}

pub trait ReportExt {
    fn attach_attribute(self, key: impl Into<Key>, value: impl Into<Value>) -> Self;
}

impl<C: Sized + 'static> ReportExt for Report<C, Mutable, SendSync> {
    fn attach_attribute(self, key: impl Into<Key>, value: impl Into<Value>) -> Self {
        self.attach_custom::<OTelAttribute, _>(KeyValue::new(key, value))
    }
}

impl<C: Sized + 'static> ReportExt for Report<C, Mutable, Local> {
    fn attach_attribute(self, key: impl Into<Key>, value: impl Into<Value>) -> Self {
        self.attach_custom::<OTelAttribute, _>(KeyValue::new(key, value))
    }
}

pub trait OTelReportExt: Sized {
    type Builder: ExceptionEventBuilder + ExceptionEventConfig;

    fn otel_raw(self) -> Self::Builder;

    fn otel(self) -> Self::Builder {
        self.otel_raw().ex_type().timestamped().backtrace()
    }

    fn otel_spec(self, spec: ExceptionEventSpec) -> Self::Builder {
        self.otel_raw().config(spec)
    }
}

impl<C, O, T> OTelReportExt for Report<C, O, T>
where
    C: 'static,
    O: 'static + ReportOwnershipMarker,
    T: 'static,
{
    type Builder = ReportWrapper<C, O, T>;

    fn otel_raw(mut self) -> Self::Builder {
        ReportWrapper::new(self)
    }
}

impl<X, C, O, T> OTelReportExt for Result<X, Report<C, O, T>>
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
