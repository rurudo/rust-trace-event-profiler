use crate::Argument;
use crate::TraceEvent;

#[derive(Clone)]
pub enum Phase {
    DurationBegin,
    DurationEnd,
    Complete,
    Metadata,
    FlowBegin,
    FlowEnd,
}

#[allow(dead_code)]
#[derive(Builder)]
#[builder(pattern = "immutable", setter(into))]
pub struct Event {
    phase: Phase,
    name: String,
    category: String,
    id: u64,
    process_id: u32,
    timestamp: u64,
    thread_id: u64,
    duration: u64,
    argument: Argument,
}

fn show_error<VALUE: Into<String>>(name: VALUE) -> String {
    format!("{} must be initialized", name.into())
}

//#[allow(dead_code)]
impl EventBuilder {
    pub fn build_duration_begin(self) -> Result<TraceEvent, String> {
        let name = self.name.ok_or(show_error("DurationBegin::name"))?;
        let pid = self
            .process_id
            .ok_or(show_error("DurationBegin::process_id"))?;
        let ts = self
            .timestamp
            .ok_or(show_error("DurationBegin::timestamp"))?;

        Ok(TraceEvent::B {
            name,
            pid,
            ts,
            tid: self.thread_id,
        })
    }

    pub fn build_duration_end(self) -> Result<TraceEvent, String> {
        let pid = self
            .process_id
            .ok_or(show_error("DurationEnd::process_id"))?;
        let ts = self.timestamp.ok_or(show_error("DurationEnd::timestamp"))?;

        Ok(TraceEvent::E {
            pid,
            ts,
            tid: self.thread_id,
        })
    }

    pub fn build_complete(self) -> Result<TraceEvent, String> {
        let name = self.name.ok_or(show_error("Complete::name"))?;
        let pid = self.process_id.ok_or(show_error("Complete::process_id"))?;
        let ts = self.timestamp.ok_or(show_error("Complete::timestamp"))?;
        let dur = self.duration.ok_or(show_error("Complete::duration"))?;

        Ok(TraceEvent::X {
            name,
            pid,
            ts,
            dur,
            tid: self.thread_id,
        })
    }

    pub fn build_metadata(self) -> Result<TraceEvent, String> {
        let name = self.name.ok_or(show_error("Metadata::name"))?;
        let pid = self.process_id.ok_or(show_error("Metadata::process_id"))?;
        let args = self.argument.ok_or(show_error("Metadata::argument"))?;

        Ok(TraceEvent::M {
            name,
            pid,
            args,
            tid: self.thread_id,
        })
    }

    pub fn build_flow_begin(self) -> Result<TraceEvent, String> {
        let name = self.name.ok_or(show_error("FlowBegin::name"))?;
        let cat = self.category.ok_or(show_error("FlowBegin::category"))?;
        let pid = self.process_id.ok_or(show_error("FlowBegin::process_id"))?;
        let tid = self.thread_id.ok_or(show_error("FlowBegin::thread_id"))?;
        let ts = self.timestamp.ok_or(show_error("FlowBegin::timestamp"))?;
        let id = self.id.ok_or(show_error("FlowBegin::id"))?;

        Ok(TraceEvent::s {
            name,
            cat,
            id,
            pid,
            tid,
            ts,
        })
    }

    pub fn build_flow_end(self) -> Result<TraceEvent, String> {
        let name = self.name.ok_or(show_error("FlowEnd::name"))?;
        let cat = self.category.ok_or(show_error("FlowEnd::category"))?;
        let pid = self.process_id.ok_or(show_error("FlowEnd::process_id"))?;
        let tid = self.thread_id.ok_or(show_error("FlowEnd::thread_id"))?;
        let ts = self.timestamp.ok_or(show_error("FlowEnd::timestamp"))?;
        let id = self.id.ok_or(show_error("FlowEnd::id"))?;

        Ok(TraceEvent::f {
            name,
            cat,
            id,
            pid,
            tid,
            ts,
        })
    }

    pub fn build_for_trace_event(self) -> Result<TraceEvent, String> {
        let phase = Clone::clone(self.phase.as_ref().ok_or(show_error("phase"))?);

        match phase {
            Phase::DurationBegin => self.build_duration_begin(),
            Phase::DurationEnd => self.build_duration_end(),
            Phase::Complete => self.build_complete(),
            Phase::Metadata => self.build_metadata(),
            Phase::FlowBegin => self.build_flow_begin(),
            Phase::FlowEnd => self.build_flow_end(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_begin() {
        let memory = EventBuilder::default()
            .phase(Phase::DurationBegin)
            .name("Task A")
            .timestamp(10_u32)
            .process_id(0_u32)
            .build_for_trace_event()
            .unwrap();
        let json = r#"{
  "ph": "B",
  "name": "Task A",
  "pid": 0,
  "ts": 10
}"#;
        let serialized = serde_json::to_string_pretty(&memory).unwrap();
        assert_eq!(serialized, json);

        let deserialized = serde_json::from_str::<TraceEvent>(json).unwrap();
        assert_eq!(deserialized, memory);
    }
}
