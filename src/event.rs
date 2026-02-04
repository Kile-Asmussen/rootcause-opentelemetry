use std::{borrow::Cow, time::SystemTime};

use opentelemetry::{
    Context, KeyValue,
    trace::{Span, SpanRef, TraceContextExt},
};
use opentelemetry_semantic_conventions::attribute;
use rootcause::{
    Report, ReportMut, ReportRef, hooks::builtin_hooks::location::Location,
    markers::ReportOwnershipMarker,
};
use rootcause_backtrace::Backtrace;

use crate::attachments::AttachmentsExt;

pub trait SpanExt {
    fn record_event(self, ev: &impl Event);
}

impl<'a> SpanExt for SpanRef<'a> {
    fn record_event(self, ev: &impl Event) {
        if let Some(ts) = ev.timestamp() {
            self.add_event_with_timestamp(ev.name(), ts, ev.attributes());
        } else {
            self.add_event(ev.name(), ev.attributes());
        }
    }
}

impl<'a, S: Span> SpanExt for &'a mut S {
    fn record_event(self, ev: &impl Event) {
        if let Some(ts) = ev.timestamp() {
            self.add_event_with_timestamp(ev.name(), ts, ev.attributes());
        } else {
            self.add_event(ev.name(), ev.attributes());
        }
    }
}

pub trait Event: Sized {
    fn name(&self) -> impl Into<Cow<'static, str>>;
    fn timestamp(&self) -> Option<SystemTime>;
    fn attributes(&self) -> Vec<KeyValue>;

    fn record_with_current_span(&self) {
        Context::current().span().record_event(self);
    }
}

impl<'a, C: 'static + ?Sized, O: 'static, T: 'static> Event for ReportRef<'a, C, O, T> {
    fn name(&self) -> impl Into<Cow<'static, str>> {
        "exception"
    }

    fn timestamp(&self) -> Option<SystemTime> {
        self.find_attachment_inner::<SystemTime>().cloned()
    }

    fn attributes(&self) -> Vec<KeyValue> {
        vec![
            KeyValue::new(attribute::EXCEPTION_TYPE, self.current_context_type_name()),
            KeyValue::new(
                attribute::EXCEPTION_MESSAGE,
                self.format_current_context().to_string(),
            ),
            KeyValue::new(attribute::EXCEPTION_STACKTRACE, self.to_string()),
        ]
    }
}

impl<C: 'static + ?Sized, O: 'static + ReportOwnershipMarker, T: 'static> Event
    for Report<C, O, T>
{
    fn name(&self) -> impl Into<Cow<'static, str>> {
        "exception"
    }

    fn timestamp(&self) -> Option<SystemTime> {
        self.as_ref().timestamp()
    }

    fn attributes(&self) -> Vec<KeyValue> {
        self.as_ref().attributes()
    }
}

impl<'a, C: 'static + ?Sized, T: 'static> Event for ReportMut<'a, C, T> {
    fn name(&self) -> impl Into<Cow<'static, str>> {
        "exception"
    }

    fn timestamp(&self) -> Option<SystemTime> {
        self.as_ref().timestamp()
    }

    fn attributes(&self) -> Vec<KeyValue> {
        self.as_ref().attributes()
    }
}

pub struct GranularExceptions<'a>(ReportRef<'a>);

impl<'a> Event for GranularExceptions<'a> {
    fn name(&self) -> impl Into<Cow<'static, str>> {
        "exception"
    }

    fn timestamp(&self) -> Option<SystemTime> {
        self.0.timestamp()
    }

    fn attributes(&self) -> Vec<KeyValue> {
        let mut res = vec![
            KeyValue::new(
                attribute::EXCEPTION_TYPE,
                self.0.current_context_type_name(),
            ),
            KeyValue::new(
                attribute::EXCEPTION_MESSAGE,
                self.0.format_current_context().to_string(),
            ),
        ];

        if let Some(bt) = self.0.find_attachment::<Backtrace>() {
            res.push(KeyValue::new(
                attribute::EXCEPTION_STACKTRACE,
                bt.to_string(),
            ));
        }

        if let Some(loc) = self.0.find_attachment::<Location>() {
            res.push(KeyValue::new("exception.location", loc.to_string()));
        }

        let mut attachments = self
            .0
            .attachments()
            .iter()
            .map(|r| {
                (
                    r,
                    r.preferred_formatting_style(rootcause::handlers::FormattingFunction::Display),
                )
            })
            .collect::<Vec<_>>();

        attachments.sort_by_key(|(_, p)| -p.priority);

        res
    }

    fn record_with_current_span(&self) {
        for c in self.0.children() {
            GranularExceptions(c).record_with_current_span();
        }
        Context::current().span().record_event(self)
    }
}
