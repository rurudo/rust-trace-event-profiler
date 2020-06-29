use serde::{Deserialize, Serialize};

use crate::TraceEvent;

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[allow(non_snake_case)]
pub struct TraceEventFormat {
    traceEvents: Vec<TraceEvent>,
}

impl TraceEventFormat {
    pub fn new(trace_events: Vec<TraceEvent>) -> TraceEventFormat {
        TraceEventFormat {
            traceEvents: trace_events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::event_builder::Phase;
    use crate::EventBuilder;

    #[test]
    fn simple() {
        let task_a = EventBuilder::default()
            .phase(Phase::Complete)
            .name("task A")
            .timestamp(10_u32)
            .duration(50_u32)
            .process_id(0_u32)
            .build_for_trace_event()
            .unwrap();

        let memory = TraceEventFormat::new(vec![task_a]);
        let serialized = serde_json::to_string_pretty(&memory).unwrap();

        let json = r#"{
  "traceEvents": [
    {
      "ph": "X",
      "name": "task A",
      "pid": 0,
      "ts": 10,
      "dur": 50
    }
  ]
}"#;
        assert_eq!(serialized, json);

        let deserialized = serde_json::from_str::<TraceEventFormat>(json).unwrap();
        assert_eq!(deserialized, memory);
    }
}
