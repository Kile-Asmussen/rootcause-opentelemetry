use std::{any::TypeId, fmt::Display, marker::PhantomData, time::SystemTime};

use opentelemetry::{KeyValue, StringValue};
use rootcause::{ReportRef, handlers::AttachmentHandler, hooks::builtin_hooks::location::Location};
use rootcause_backtrace::{Backtrace, BacktraceHandler};

#[derive(Debug, Default)]
pub(crate) struct EventAttachments {
    pub(crate) timestamp: Option<SystemTime>,
    pub(crate) backtrace: Option<String>, // TODO!!!
    pub(crate) location: Option<Location>,
    pub(crate) all: Vec<(TypeId, String)>,
    pub(crate) keyvals: Vec<KeyValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AttachmentAction {
    Smart(),
    All(),
    OfType(TypeId),
}

impl EventAttachments {
    pub(crate) fn from<C: 'static, O: 'static, T: 'static>(rep: ReportRef<C, O, T>) -> Self {
        let mut res = Self::default();

        for at in rep.attachments() {
            if let Some(st) = at.downcast_inner::<SystemTime>() {
                if res.timestamp.is_none() {
                    res.timestamp = Some(*st);
                }
            }

            if let Some(lt) = at.downcast_inner::<Location>() {
                if res.location.is_none() {
                    res.location = Some(lt.clone())
                }
            }

            if let Some(kv) = at.downcast_inner::<KeyValue>() {
                res.keyvals.push(kv.clone());
            }

            if let Some(bt) = at.downcast_attachment::<Backtrace>() {
                if res.backtrace.is_none() {
                    res.backtrace = Some(bt.format_inner().to_string())
                }
            }

            res.all
                .push((at.inner_type_id(), at.format_inner().to_string()));
        }

        return res;
    }

    pub(crate) fn list_into_vec(
        &self,
        actions: impl IntoIterator<Item = AttachmentAction>,
        res: &mut Vec<StringValue>,
    ) {
        for at in actions {
            match at {
                AttachmentAction::Smart() => self.list_smartly(res),
                AttachmentAction::All() => self.list_all(res),
                AttachmentAction::OfType(type_id) => self.list_by_type_id(type_id, res),
            }
        }
    }

    pub(crate) fn list_all(&self, res: &mut Vec<StringValue>) {
        for (_, s) in &self.all {
            res.push(s.clone().into())
        }
    }

    pub(crate) fn list_smartly(&self, res: &mut Vec<StringValue>) {
        let mut ts_seen = false;
        let mut bt_seen = false;
        let mut lt_seen = false;

        for (id, s) in &self.all {
            if id == &TypeId::of::<SystemTime>() {
                if ts_seen {
                    res.push(s.clone().into())
                } else {
                    ts_seen = true;
                }
            } else if id == &TypeId::of::<Backtrace>() {
                if bt_seen {
                    res.push(s.clone().into())
                } else {
                    bt_seen = true;
                }
            } else if id == &TypeId::of::<Location>() {
                if lt_seen {
                    res.push(s.clone().into());
                } else {
                    lt_seen = true;
                }
            } else if id != &TypeId::of::<KeyValue>() {
                res.push(s.clone().into())
            }
        }
    }

    pub(crate) fn list_by_type_id(&self, type_id: TypeId, res: &mut Vec<StringValue>) {
        for (id, s) in &self.all {
            if id == &type_id {
                res.push(s.clone().into())
            }
        }
    }
}
