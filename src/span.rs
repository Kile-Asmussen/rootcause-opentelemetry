use opentelemetry::trace::{Span, SpanRef};

trait SpanRefExt {
    fn send_report_event(self);
}

trait SpanTraitExt {
}

fn test() {
    let x : &dyn Span;
}