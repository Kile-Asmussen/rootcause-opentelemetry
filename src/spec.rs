use std::{any::TypeId, time::SystemTime};

use opentelemetry::StringValue;

/// Trait for configuring Open Telemetry exception events
pub trait OTelEventConfig: Sized {
    /// Derive the `exception.type` attribute automatically
    fn ex_type(self) -> Self;
    /// Specify the `exception.type` attribute directly
    fn set_ex_type(self, ex_type: impl Into<StringValue>) -> Self;

    /// Specify the `exception.message` attribute directly
    /// 
    /// By default it is the formatted context
    fn custom_message(self, msg: impl Into<StringValue>) -> Self;

    /// Derive the event timestamp from the report attachments
    fn timestamped(self) -> Self;
    /// Specify the event timestamp
    fn set_timestamp(self, systime: SystemTime) -> Self;
    /// Set the event timestamp to now
    fn timestamp_now(self) -> Self;

    /// Derive the `exception.stacktrace` from the backtrace attachment
    fn backtrace(self) -> Self;

    /// Specify the `exception.escaped` attribute
    fn escaped(self, has_escaped: bool) -> Self;

    /// Include a custom element in the `exception.extra` attribute
    fn add_attacment(self, at: impl Into<StringValue>) -> Self;

    /// Include a custom element in the `exception.extra` attribute
    fn all_attachments(self) -> Self;

    /// Include all attachments that are not `Backtrace`` and `SystemTime`
    /// in the `exception.extra` attribute
    fn attachments(self) -> Self;

    /// Include all attachments that of the given type ID
    /// in the `exception.extra` attribute
    fn attachments_of_type_id(self, type_id: TypeId) -> Self;

    /// Convenience function for [`attachments_of_type_id`]
    fn attachments_of_type<T: 'static>(self) -> Self {
        self.attachments_of_type_id(TypeId::of::<T>())
    }
    
    /// Use the current configuration to create events for
    /// all child reports
    fn recurse(self) -> Self;

    /// Use the given configuration to create events for
    /// the immediate child reports
    fn children(self, actions: OTelEventSpec) -> Self;
}

#[derive(Default, Debug, PartialEq)]
pub struct OTelEventSpec {
    ex_type: Option3<StringValue>,
    custom_message: Option<StringValue>,
    timestamp: Option3<Option<SystemTime>>,
    backtrace: bool,
    escaped: Option<bool>,
    attachments: Vec<AttachmentAction>,
    children: Option3<Box<OTelEventSpec>>
}

impl OTelEventSpec {
    fn inject<OT: OTelEventConfig>(self, mut other: OT) -> OT {
        
        other = match self.ex_type {
            Option3::Default => other,
            Option3::Inferred => other.ex_type(),
            Option3::Specific(s) => other.set_ex_type(s),
        };

        other = if let Some(msg) = self.custom_message {
            other.custom_message(msg)
        } else {
            other
        };

        other = match self.timestamp {
            Option3::Default => other,
            Option3::Inferred => other.timestamped(),
            Option3::Specific(None) => other.timestamp_now(),
            Option3::Specific(Some(st)) => other.set_timestamp(st),
        };

        other = if self.backtrace {
            other.backtrace()
        } else {
            other
        };

        other = match self.escaped {
            Some(has_escaped) => other.escaped(has_escaped),
            _ => other
        };

        for at in self.attachments {
            other = match at {
                AttachmentAction::Smart() => other.attachments(),
                AttachmentAction::All() => other.all_attachments(),
                AttachmentAction::Custom(string_value) => other.add_attacment(string_value),
                AttachmentAction::OfType(type_id) => other.attachments_of_type_id(type_id),
            }
        }

        other = match self.children {
            Option3::Default => other,
            Option3::Inferred => other.recurse(),
            Option3::Specific(builder) => other.children(*builder),
        };

        other
    }
}

#[derive(Default, Debug, PartialEq)]
enum Option3<T> {
    #[default]
    Default,
    Inferred,
    Specific(T)
}

#[derive(Debug, PartialEq)]
enum AttachmentAction {
    Smart(),
    All(),
    Custom(StringValue),
    OfType(TypeId),
}

impl OTelEventConfig for &mut OTelEventSpec {
    fn escaped(self, has_escaped: bool) -> Self {
        self.escaped = Some(has_escaped);
        self
    }

    fn ex_type(self) -> Self {
        self.ex_type = Option3::Inferred;
        self
    }

    fn set_ex_type(self, ex_type: impl Into<StringValue>) -> Self {
        self.ex_type = Option3::Specific(ex_type.into());
        self
    }

    fn backtrace(self) -> Self {
        self.backtrace = true;
        self
    }

    fn timestamped(self) -> Self {
        self.timestamp = Option3::Inferred;
        self
    }

    fn set_timestamp(self, systime: SystemTime) -> Self {
        self.timestamp = Option3::Specific(Some(systime));
        self
    }

    fn timestamp_now(self) -> Self {
        self.timestamp = Option3::Specific(None);
        self
    }

    fn recurse(self) -> Self {
        self.children = Option3::Inferred;
        self
    }

    fn add_attacment(self, at: impl Into<StringValue>) -> Self {
        self.attachments.push(AttachmentAction::Custom(at.into()));
        self
    }

    fn all_attachments(self) -> Self {
        self.attachments.push(AttachmentAction::All());
        self
    }

    fn attachments(self) -> Self {
        self.attachments.push(AttachmentAction::Smart());
        self
    }

    fn attachments_of_type_id(self, type_id: TypeId) -> Self {
        self.attachments.push(AttachmentAction::OfType(type_id));
        self
    }

    fn children(self, actions: OTelEventSpec) -> Self {
        self.children = Option3::Specific(Box::new(actions));
        self
    }
    
    fn custom_message(self, msg: impl Into<StringValue>) -> Self {
        self.custom_message = Some(msg.into());
        self
    }
}


impl OTelEventConfig for OTelEventSpec {
    fn ex_type(mut self) -> Self {
        (&mut self).ex_type(); self
    }

    fn set_ex_type(mut self, ex_type: impl Into<StringValue>) -> Self {
        (&mut self).set_ex_type(ex_type); self
    }

    fn custom_message(mut self, msg: impl Into<StringValue>) -> Self {
        (&mut self).custom_message(msg); self
    }

    fn timestamped(mut self) -> Self {
        (&mut self).timestamped(); self
    }

    fn set_timestamp(mut self, systime: SystemTime) -> Self {
        (&mut self).set_timestamp(systime); self
    }

    fn timestamp_now(mut self) -> Self {
        (&mut self).timestamp_now(); self
    }

    fn backtrace(mut self) -> Self {
        (&mut self).backtrace(); self
    }

    fn escaped(mut self, has_escaped: bool) -> Self {
        (&mut self).escaped(has_escaped); self
    }

    fn add_attacment(mut self, at: impl Into<StringValue>) -> Self {
        (&mut self).add_attacment(at); self
    }

    fn all_attachments(mut self) -> Self {
        (&mut self).all_attachments(); self
    }

    fn attachments(mut self) -> Self {
        (&mut self).attachments(); self
    }

    fn attachments_of_type_id(mut self, type_id: TypeId) -> Self {
        (&mut self).attachments_of_type_id(type_id); self
    }

    fn recurse(mut self) -> Self {
        (&mut self).recurse(); self
    }

    fn children(mut self, actions: OTelEventSpec) -> Self {
        (&mut self).children(actions); self
    }
}

pub trait OTelEventConfigExt {
    fn config(self, spec: OTelEventSpec) -> Self;
}

impl<OT: OTelEventConfig> OTelEventConfigExt for OT {
    fn config(self, spec: OTelEventSpec) -> Self {
        spec.inject(self)
    }
}

