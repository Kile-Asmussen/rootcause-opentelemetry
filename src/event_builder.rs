use std::{alloc::System, any, backtrace, time::SystemTime};

use chrono::Local;
use opentelemetry::{Context, KeyValue, StringValue, trace::TraceContextExt};
use opentelemetry_sdk::trace::SpanEvents;
use opentelemetry_semantic_conventions::trace::{
    EXCEPTION_MESSAGE, EXCEPTION_STACKTRACE, EXCEPTION_TYPE,
};
use rootcause::{
    Report, ReportRef,
    hooks::builtin_hooks::location::{self, Location},
    markers::{Mutable, ReportOwnershipMarker},
};
use rootcause_backtrace::Backtrace;

use crate::attachments::SentTo;

#[allow(type_alias_bounds)]
type StringProducer<C, O: ReportOwnershipMarker, T> =
    fn(ReportRef<C, O::RefMarker, T>) -> Option<StringValue>;
#[allow(type_alias_bounds)]
type AttributesProducer<C, O: ReportOwnershipMarker, T> =
    fn(ReportRef<C, O::RefMarker, T>) -> Vec<KeyValue>;
#[allow(type_alias_bounds)]
type TimestampProducer<C, O: ReportOwnershipMarker, T> =
    fn(ReportRef<C, O::RefMarker, T>) -> Option<SystemTime>;
#[allow(type_alias_bounds)]
type BacktraceProducer<C, O: ReportOwnershipMarker, T> =
    fn(ReportRef<C, O::RefMarker, T>) -> Option<StringValue>;
#[allow(type_alias_bounds)]
type ReportRecursor<C, O: ReportOwnershipMarker, T> =
    fn(ReportRef<C, O::RefMarker, T>) -> Vec<ReportRef<C, O::RefMarker, T>>;
#[allow(type_alias_bounds)]
type EventBuilderProducer<C, O: ReportOwnershipMarker, T> =
    fn(ReportRef<C, O::RefMarker, T>) -> EventConfig<C, O, T>;

#[derive(Debug)]
pub struct EventConfig<C, O, T>
where
    C: 'static,
    O: 'static + ReportOwnershipMarker,
    T: 'static,
{
    type_name: StringProducer<C, O, T>,
    message: StringProducer<C, O, T>,
    timestamp: TimestampProducer<C, O, T>,
    backtrace: StringProducer<C, O, T>,
    attribute_producers: Vec<AttributesProducer<C, O, T>>,
    recursors: Vec<(ReportRecursor<C, O, T>, EventBuilderProducer<C, O, T>)>,
}

#[derive(Default, Debug)]
struct SpanExceptionEvent {
    timestamp: Option<SystemTime>,
    attributes: Vec<KeyValue>,
}

impl<C, O, T> EventConfig<C, O, T>
where
    C: 'static,
    O: 'static + ReportOwnershipMarker,
    T: 'static,
{
    fn default_type_name(r: ReportRef<C, O::RefMarker, T>) -> Option<StringValue> {
        Some(any::type_name::<C>().into()) // TODO
    }

    fn display_message(r: ReportRef<C, O::RefMarker, T>) -> Option<StringValue> {
        Some(format!("{}", r.format_current_context()).into())
    }

    fn default_find_timestamp(r: ReportRef<C, O::RefMarker, T>) -> Option<SystemTime> {
        for at in r.attachments() {
            let res = at.downcast_inner().map(|x| *x);
            if res.is_some() {
                return res;
            }
        }
        None
    }

    fn default_find_backtrace(r: ReportRef<C, O::RefMarker, T>) -> Option<StringValue> {
        let mut backtrace = None;
        let mut location = None;
        for at in r.attachments() {
            backtrace = backtrace.or(at.downcast_attachment::<Backtrace>());
            location = location.or(at.downcast_attachment::<Location>());
        }
        let backtrace = backtrace.map(|a| a.format_inner().to_string().into());
        let location = location.map(|a| a.format_inner().to_string().into());
        return backtrace.or(location);
    }

    fn find_sent_to(r: ReportRef<C, O::RefMarker, T>) -> Option<SentTo> {
        r.attachments()
            .iter()
            .find_map(|a| a.downcast_inner().map(|s| *s))
    }

    pub fn new() -> Self {
        Self {
            type_name: Self::default_type_name,
            message: Self::display_message,
            timestamp: Self::default_find_timestamp,
            backtrace: Self::default_find_backtrace,
            attribute_producers: vec![],
            recursors: vec![],
        }
    }

    fn build(&self, rep: ReportRef<C, O::RefMarker, T>, res: &mut Vec<SpanExceptionEvent>) {
        for (rec, conf) in self.recursors.iter().map(Clone::clone) {
            let subs = rec(rep);
            for sub in subs {
                let conf = conf(sub);
                conf.build(sub, res);
            }
        }

        let mut this = SpanExceptionEvent::default();
        let mut ok = false;
        this.timestamp = (self.timestamp)(rep);
        if let Some(msg) = (self.message)(rep) {
            ok = true;
            this.attributes.push(KeyValue::new(EXCEPTION_MESSAGE, msg));
        }
        if let Some(ty) = (self.type_name)(rep) {
            ok = true;
            this.attributes.push(KeyValue::new(EXCEPTION_TYPE, ty));
        }
        if let Some(back) = (self.backtrace)(rep) {
            this.attributes
                .push(KeyValue::new(EXCEPTION_STACKTRACE, back));
        }
        if let Some(_) = Self::find_sent_to(rep) {
            ok = false;
        }
        if ok {
            res.push(this);
        }
    }
}

struct EventBuilder<C, O, T>
where
    C: 'static,
    O: 'static + ReportOwnershipMarker,
    T: 'static,
{
    rep: Report<C, O, T>,
    config: EventConfig<C, O, T>,
}

impl<C, O, T> EventBuilder<C, O, T>
where
    C: 'static,
    O: 'static + ReportOwnershipMarker,
    T: 'static,
{
    fn new(rep: Report<C, O, T>) -> Self {
        Self {
            rep,
            config: EventConfig::new(),
        }
    }
}

impl<C, O, T> OTelBuilder for EventBuilder<C, O, T>
where
    C: 'static,
    O: 'static + ReportOwnershipMarker,
    T: 'static,
{
    type Context = C;

    type Ownership = O;

    type ThreadSafety = T;

    type WrappedReport = Report<C, O, T>;

    fn send(self) -> Self::WrappedReport {
        let mut evs = vec![];
        self.config.build(self.rep.as_ref(), &mut evs);
        let ctx = Context::current();
        let span = ctx.span();
        for ev in evs {
            if let Some(ts) = ev.timestamp {
                span.add_event_with_timestamp("exception", ts, ev.attributes);
            } else {
                span.add_event("exception", ev.attributes);
            }
        }
        self.rep
    }
}

trait OTelBuilder {
    type Context: 'static;
    type Ownership: 'static + ReportOwnershipMarker;
    type ThreadSafety: 'static;
    type WrappedReport;
    fn send(self) -> Self::WrappedReport;
}

impl<X, OT: OTelBuilder> OTelBuilder for Result<X, OT> {
    type Context = OT::Context;

    type Ownership = OT::Ownership;

    type ThreadSafety = OT::ThreadSafety;

    type WrappedReport = Result<X, OT::WrappedReport>;

    fn send(self) -> Self::WrappedReport {
        self.map_err(|e| e.send())
    }
}

trait OTelExt {
    type Contenxt: 'static;
    type Ownership: 'static + ReportOwnershipMarker;
    type ThreadSafety: 'static;
    type Builder: OTelBuilder<
            Context = Self::Contenxt,
            Ownership = Self::Ownership,
            ThreadSafety = Self::ThreadSafety,
            WrappedReport = Self,
        >;

    fn otel(self) -> Self::Builder;
}

impl<C, O, T> OTelExt for Report<C, O, T>
where
    C: 'static,
    O: 'static + ReportOwnershipMarker,
    T: 'static,
{
    type Contenxt = C;

    type Ownership = O;

    type ThreadSafety = T;

    type Builder = EventBuilder<C, O, T>;

    fn otel(self) -> Self::Builder {
        EventBuilder::new(self)
    }
}

impl<X, C, O, T> OTelExt for Result<X, Report<C, O, T>>
where
    C: 'static,
    O: 'static + ReportOwnershipMarker,
    T: 'static,
{
    type Contenxt = C;

    type Ownership = O;

    type ThreadSafety = T;

    type Builder = Result<X, EventBuilder<C, O, T>>;

    fn otel(self) -> Self::Builder {
        self.map_err(EventBuilder::new)
    }
}
