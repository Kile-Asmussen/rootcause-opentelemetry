
use std::time::SystemTime;

use opentelemetry::{Array, KeyValue, StringValue, Value, trace::{Span, SpanRef}};
use opentelemetry_semantic_conventions::trace;
use rootcause::{ReportRef, handlers::AttachmentHandler, hooks::builtin_hooks::location::{self, LocationHandler}};

use crate::{attachments::EventAttachments, spec::{ExceptionEventSpec, Option3}, utils::Displayable};

const EXCEPTION : &'static str = "exception";
const EXCEPTION_EXTRAS : &'static str = "exception.extras";
const EXCEPTION_ORIGIN : &'static str = "exception.origin";

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
            (Option3::Default, _) | (Option3::Inferred, None) => self.add_event(EXCEPTION, attributes),
            (Option3::Specific(None), _) => self.add_event_with_timestamp(EXCEPTION, SystemTime::now(), attributes),
            (Option3::Inferred, Some(st)) | (Option3::Specific(Some(st)), _)
                => self.add_event_with_timestamp(EXCEPTION,  st, attributes),
        }
    }
}

impl<S: Span> SpanTraitExt for S {
    fn add_exception_event<C: 'static, O: 'static, T: 'static>(&mut self, rep: ReportRef<C, O, T>, spec: ExceptionEventSpec) {
        let attachments = EventAttachments::from(rep);
        let timestamp_spec = spec.timestamp.clone();
        let attributes = attributes(rep, &attachments, spec);

        match (timestamp_spec, attachments.timestamp) {
            (Option3::Default, _) | (Option3::Inferred, None) => self.add_event(EXCEPTION, attributes),
            (Option3::Inferred, Some(st)) => self.add_event_with_timestamp(EXCEPTION,  st, attributes),
            (Option3::Specific(None), _) => self.add_event_with_timestamp(EXCEPTION, SystemTime::now(), attributes),
            (Option3::Specific(Some(st)), _) => self.add_event_with_timestamp(EXCEPTION, st, attributes),
        }
    }
}

fn attributes<C: 'static, O: 'static, T: 'static>(rep: ReportRef<C, O, T>, attachments: &EventAttachments, mut spec: ExceptionEventSpec) -> Vec<KeyValue> {
    let mut res = Vec::<KeyValue>::with_capacity(4);
    
    match spec.ex_type {
        Option3::Default => {},
        Option3::Inferred => {
            res.push(KeyValue::new(trace::EXCEPTION_TYPE, "Report<Dynamic>")); // TODO!!!
        },
        Option3::Specific(ex_type) => {
            res.push(KeyValue::new(trace::EXCEPTION_TYPE, ex_type));
        },
    }

    if let Some(msg) = spec.custom_message {
        res.push(KeyValue::new(trace::EXCEPTION_MESSAGE, msg));
    } else {
        res.push(KeyValue::new(trace::EXCEPTION_MESSAGE, rep.format_current_context().to_string()))
    }

    match (spec.backtrace, &attachments.backtrace) {
        (Option3::Default, _) | (Option3::Inferred, None) => {},
        (Option3::Inferred, Some(backtrace)) =>
            res.push(KeyValue::new(trace::EXCEPTION_STACKTRACE, backtrace.clone())),
        (Option3::Specific(backtrace), _) =>
            res.push(KeyValue::new(trace::EXCEPTION_STACKTRACE, backtrace)),
    }

    let mut extras: Vec<StringValue> = vec![];
    attachments.list_into_vec(spec.attachments, &mut extras);

    if extras.len() > 0 {
        res.push(KeyValue::new(EXCEPTION_EXTRAS, Value::Array(Array::String(extras))))
    }

    match (spec.location, attachments.location) {
        (Option3::Default, _) | (Option3::Inferred, None) => {},
        (Option3::Inferred, Some(loc)) | (Option3::Specific(loc), _) => {
            res.push(KeyValue::new(EXCEPTION_ORIGIN, loc.displayable().to_string()));
        }
    }

    if spec.attributes {
        for attr in &attachments.keyvals {
            res.push(attr.clone())
        }
    }

    res.extend(spec.extra_attributes);

    res
}