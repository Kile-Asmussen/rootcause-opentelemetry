use std::{fmt::Display, marker::PhantomData};

use rootcause::{handlers::AttachmentHandler, hooks::builtin_hooks::location::{Location, LocationHandler}};
use rootcause_backtrace::{Backtrace, BacktraceHandler};


struct DisplayMe<'a, H, A: 'static>(&'a A, PhantomData<H>);
impl<'a, A: 'static, H : AttachmentHandler<A>> Display for DisplayMe<'a, H, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        H::display(self.0, f)
    }
}

pub trait Displayable: Sized + 'static {
    type Handler : AttachmentHandler<Self>;
    fn displayable<'a>(&'a self) -> impl Display {
        DisplayMe(self, PhantomData::<Self::Handler>)
    }
}

impl Displayable for Location {
    type Handler = LocationHandler;
}

impl Displayable for Backtrace {
    type Handler = BacktraceHandler<false>;
}
