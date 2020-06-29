use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct Argument {
    name: String,
}

impl Argument {
    pub fn new<Name: Into<String>>(name: Name) -> Argument {
        Argument { name: name.into() }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
// event phase type
#[serde(tag = "ph")]
pub enum TraceEvent {
    // B(begin) Duration
    B {
        name: String,
        pid: u32,
        ts: u64,

        #[serde(skip_serializing_if = "Option::is_none")]
        tid: Option<u64>,
    },
    // E(end) Duration
    E {
        pid: u32,
        ts: u64,

        #[serde(skip_serializing_if = "Option::is_none")]
        tid: Option<u64>,
    },
    // Complete
    X {
        name: String,
        pid: u32,
        ts: u64,
        dur: u64,

        #[serde(skip_serializing_if = "Option::is_none")]
        tid: Option<u64>,
    },
    // I(instant)
    I {},
    // C(counter)
    C {},
    // b(nestable start) Async
    #[allow(non_camel_case_types)]
    b {},
    // n(nestable instant) Async
    #[allow(non_camel_case_types)]
    n {},
    // e(nestable end) Async
    #[allow(non_camel_case_types)]
    e {},
    // s(start) Flow
    #[allow(non_camel_case_types)]
    s {
        name: String,
        pid: u32,
        tid: u64,
        ts: u64,
        cat: String,
        id: u64,
    },
    // t(step) Flow
    #[allow(non_camel_case_types)]
    t {},
    // f(end) Flow
    #[allow(non_camel_case_types)]
    f {
        name: String,
        pid: u32,
        tid: u64,
        ts: u64,
        cat: String,
        id: u64,
    },
    // Sample
    P {},
    // N(created) Object
    N {},
    // O(snapshot) Object
    O {},
    // D(destroyed) Object
    D {},
    // M(metadata)
    M {
        name: String,
        pid: u32,
        args: Argument,

        #[serde(skip_serializing_if = "Option::is_none")]
        tid: Option<u64>,
    },
    // V(global) Memory Dump
    V {},
    // v(process) Memory Dump
    #[allow(non_camel_case_types)]
    v {},
    // Mark
    R {},
    // Clock Sync
    #[allow(non_camel_case_types)]
    c {},
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_begin() {
        let memory = TraceEvent::B {
            name: "Task A".to_string(),
            pid: 1,
            ts: 10,
            tid: None,
        };
        let json = r#"{
  "ph": "B",
  "name": "Task A",
  "pid": 1,
  "ts": 10
}"#;
        let serialized = serde_json::to_string_pretty(&memory).unwrap();
        assert_eq!(serialized, json);

        let deserialized = serde_json::from_str::<TraceEvent>(json).unwrap();
        assert_eq!(deserialized, memory);
    }
}
