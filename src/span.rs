
use std::{backtrace, time::SystemTime};

use opentelemetry::{Array, KeyValue, StringValue, Value, trace::{Span, SpanRef}};
use rootcause::ReportRef;

use crate::{attachments::EventAttachments, spec::{ExceptionEventSpec, Option3}};

pub(crate) trait SpanRefExt: Sized {
    fn add_exception_event<C: 'static, O: 'static, T: 'static>(&self, rep: ReportRef<C, O, T>, spec: ExceptionEventSpec);
}

pub(crate) trait SpanTraitExt: Sized {
    fn add_exception_event<C: 'static, O: 'static, T: 'static>(&mut self, rep: ReportRef<C, O, T>, spec: ExceptionEventSpec);
}

impl<'a, 'b: 'a> SpanRefExt for SpanRef<'a> {
    fn add_exception_event<C: 'static, O: 'static, T: 'static>(&self, rep: ReportRef<C, O, T>, spec: ExceptionEventSpec)
    {
        let attachments = EventAttachments::from(rep);
        let timestamp_spec = spec.timestamp.clone();
        let attributes = attributes(rep, &attachments, spec);

        match (timestamp_spec, attachments.timestamp) {
            (Option3::Default, _) | (Option3::Inferred, None) => self.add_event("exception", attributes),
            (Option3::Inferred, Some(st)) => self.add_event_with_timestamp("exception",  st, attributes),
            (Option3::Specific(None), _) => self.add_event_with_timestamp("exception", SystemTime::now(), attributes),
            (Option3::Specific(Some(st)), _) => self.add_event_with_timestamp("exception", st, attributes),
        }
    }
}

impl<S: Span> SpanTraitExt for S {
    fn add_exception_event<C: 'static, O: 'static, T: 'static>(&mut self, rep: ReportRef<C, O, T>, spec: ExceptionEventSpec) {
        let attachments = EventAttachments::from(rep);
        let timestamp_spec = spec.timestamp.clone();
        let attributes = attributes(rep, &attachments, spec);

        match (timestamp_spec, attachments.timestamp) {
            (Option3::Default, _) | (Option3::Inferred, None) => self.add_event("exception", attributes),
            (Option3::Inferred, Some(st)) => self.add_event_with_timestamp("exception",  st, attributes),
            (Option3::Specific(None), _) => self.add_event_with_timestamp("exception", SystemTime::now(), attributes),
            (Option3::Specific(Some(st)), _) => self.add_event_with_timestamp("exception", st, attributes),
        }
    }
}

fn attributes<C: 'static, O: 'static, T: 'static>(rep: ReportRef<C, O, T>, attachments: &EventAttachments, spec: ExceptionEventSpec) -> Vec<KeyValue> {
    let mut res = Vec::<KeyValue>::with_capacity(4);
    
    match spec.ex_type {
        Option3::Default => {},
        Option3::Inferred => {
            res.push(KeyValue::new("exception.type", "Report<Dynamic>")); // TODO!!!
        },
        Option3::Specific(ex_type) => {
            res.push(KeyValue::new("exception.type", ex_type));
        },
    }

    if let Some(msg) = spec.custom_message {
        res.push(KeyValue::new("exception.message", msg));
    } else {
        res.push(KeyValue::new("exception.message", rep.format_current_context().to_string()))
    }

    match (spec.backtrace, &attachments.backtrace) {
        (Option3::Default, _) | (Option3::Inferred, None) => {},
        (Option3::Inferred, Some(backtrace)) =>
            res.push(KeyValue::new("exception.stacktrace", backtrace.clone())),
        (Option3::Specific(backtrace), _) =>
            res.push(KeyValue::new("exception.stacktrace", backtrace)),
    }

    if let Some(esc) = spec.escaped {
        res.push(KeyValue::new("exception.escaped", esc))
    }

    let mut extras: Vec<StringValue> = vec![];
    attachments.list_into_vec(spec.attachments, &mut extras);

    if extras.len() > 0 {
        res.push(KeyValue::new("exception.extras", Value::Array(Array::String(extras))))
    }

    res
}