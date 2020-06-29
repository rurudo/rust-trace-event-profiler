#[macro_use]
extern crate derive_builder;

mod event_builder;
pub mod profiler;
mod serialize_root;
mod trace_event;

pub use event_builder::EventBuilder;
pub use profiler::Profiler;
pub use serialize_root::TraceEventFormat;
pub use trace_event::Argument;
pub use trace_event::TraceEvent;
