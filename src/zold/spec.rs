use std::{any::TypeId, backtrace, time::SystemTime};

use opentelemetry::{Key, KeyValue, StringValue, Value};
use rootcause::hooks::builtin_hooks::location::Location;

use crate::attachments::AttachmentAction;

/// Trait for configuring Open Telemetry exception events
pub trait ExceptionEventConfig: Sized {
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

    /// Derive the `exception.stacktrace` from the backtrace attachment
    fn override_backtrace(self, backtrace: String) -> Self;

    /// Include a custom element in the `exception.extra` attribute
    fn all_attachments(self) -> Self;

    /// Include all attachments that are not `Backtrace`, `SystemTime`, or `KeyVal`
    /// in the `exception.extra` attribute
    fn attachments(self) -> Self; 

    /// Include attachments of type `KeyValue` in the attributes of the event
    fn attributes(self) -> Self;

    /// Include an attribute in the event
    fn add_attribute(self, key: impl Into<Key>, value: impl Into<Value>) -> Self;

    /// Include all attachments that of the given type ID
    /// in the `exception.extra` attribute
    fn attachments_of_type_id(self, type_id: TypeId) -> Self;

    /// Convenience function for [`attachments_of_type_id`]
    fn attachments_of_type<T: 'static>(self) -> Self {
        self.attachments_of_type_id(TypeId::of::<T>())
    }

    // Derive location 
    fn location(self) -> Self;
    fn set_location(self, loc: impl Into<StringValue>) -> Self;
    
    /// Use the current configuration to create events for
    /// all child reports
    fn recurse(self) -> Self;

    /// Use the given configuration to create events for
    /// the immediate child reports
    fn children(self, actions: ExceptionEventSpec) -> Self;
}

#[derive(Debug, Clone)]
pub struct ExceptionEventSpec {
    pub(crate) ex_type: Option3<StringValue>,
    pub(crate) custom_message: Option<StringValue>,
    pub(crate) location: Option3<StringValue>,
    pub(crate) timestamp: Option3<Option<SystemTime>>,
    pub(crate) backtrace: Option3<String>,
    pub(crate) attachments: Vec<AttachmentAction>,
    pub(crate) attributes: bool,
    pub(crate) extra_attributes: Vec<KeyValue>,
    pub(crate) children: Option3<Box<ExceptionEventSpec>>
}

impl Default for ExceptionEventSpec {
    fn default() -> Self {
        Self::new().with_defaults()
    }
}

impl ExceptionEventSpec {
    fn new() -> Self {
        Self {
            ex_type: Option3::Default,
            custom_message: None,
            location: Option3::Default,
            timestamp: Option3::Default,
            backtrace: Option3::Default,
            attachments: vec![],
            children: Option3::Default,
            attributes: false,
            extra_attributes: vec![],
        }
    }

    fn inject<OT: ExceptionEventConfig>(self, mut other: OT) -> OT {
        
        other = match self.ex_type {
            Option3::Default => other,
            Option3::Inferred => other.ex_type(),
            Option3::Specific(s) => other.set_ex_type(s),
        };

        if let Some(msg) = self.custom_message {
            other = other.custom_message(msg)
        }

        other = match self.timestamp {
            Option3::Default => other,
            Option3::Inferred => other.timestamped(),
            Option3::Specific(None) => other.timestamp_now(),
            Option3::Specific(Some(st)) => other.set_timestamp(st),
        };

        other = match self.backtrace {
            Option3::Default => other,
            Option3::Inferred => other.backtrace(),
            Option3::Specific(bt) => other.override_backtrace(bt),
        };

        if self.attributes {
            other = other.attributes()
        }

        for at in self.extra_attributes {
            other = other.add_attribute(at.key, at.value)
        }

        for at in self.attachments {
            other = match at {
                AttachmentAction::Smart() => other.attachments(),
                AttachmentAction::All() => other.all_attachments(),
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

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) enum Option3<T> {
    #[default]
    Default,
    Inferred,
    Specific(T)
}

impl ExceptionEventConfig for &mut ExceptionEventSpec {
    fn ex_type(self) -> Self {
        self.ex_type = Option3::Inferred;
        self
    }

    fn set_ex_type(self, ex_type: impl Into<StringValue>) -> Self {
        self.ex_type = Option3::Specific(ex_type.into());
        self
    }

    fn backtrace(self) -> Self {
        self.backtrace = Option3::Inferred;
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

    fn location(self) -> Self {
        self.location = Option3::Inferred;
        self
    }

    fn set_location(self, location: impl Into<StringValue>) -> Self {
        self.location = Option3::Specific(loc.into());
        self
    }

    fn recurse(self) -> Self {
        self.children = Option3::Inferred;
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

    fn children(self, actions: ExceptionEventSpec) -> Self {
        self.children = Option3::Specific(Box::new(actions));
        self
    }
    
    fn custom_message(self, msg: impl Into<StringValue>) -> Self {
        self.custom_message = Some(msg.into());
        self
    }
    
    fn override_backtrace(self, backtrace: String) -> Self {
        self.backtrace = Option3::Specific(backtrace);
        self
    }
    
    fn attributes(self) -> Self {
        self.attributes = true;
        self
    }
    
    fn add_attribute(self, key: impl Into<Key>, value: impl Into<Value>) -> Self {
        todo!()
    }
}


impl ExceptionEventConfig for ExceptionEventSpec {
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

    fn override_backtrace(mut self, backtrace: String) -> Self {
        (&mut self).override_backtrace(backtrace); self    
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

    fn children(mut self, actions: ExceptionEventSpec) -> Self {
        (&mut self).children(actions); self
    }
    
    fn attributes(mut self) -> Self {
        (&mut self).attributes(); self
    }
    
    fn add_attribute(mut self, key: impl Into<Key>, value: impl Into<Value>) -> Self {
        (&mut self).add_attribute(key, value); self
    }
    
    fn set_location(mut self, loc: Location) -> Self {
        (&mut self).set_location(loc); self
    }
    
    fn location(mut self) -> Self {
        (&mut self).location(); self
    }
}

pub trait ExceptionEventConfigExt {
    fn with_defaults(self) -> Self;
    fn config(self, spec: ExceptionEventSpec) -> Self;
}

impl<OT: ExceptionEventConfig> ExceptionEventConfigExt for OT {
    fn with_defaults(self) -> Self {
        self.ex_type()
            .timestamped()
            .backtrace()
    }
    fn config(self, spec: ExceptionEventSpec) -> Self {
        spec.inject(self)
    }
}

