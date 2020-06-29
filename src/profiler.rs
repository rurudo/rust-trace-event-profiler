use std::{
    convert::TryFrom,
    fs::File,
    io::Write,
    mem::transmute,
    path::Path,
    process,
    thread::{self, ThreadId},
    time::Instant,
};

use crate::event_builder::Phase;
use crate::Argument;
use crate::EventBuilder;
use crate::TraceEvent;
use crate::TraceEventFormat;

#[derive(Clone)]
pub struct Profiler {
    trace_events: Vec<TraceEvent>,
    capture_started_time: Instant,
    current_flow_id: u64,
}

fn current_thread_id() -> u64 {
    // error[E0658]: use of unstable library feature 'thread_id_value'
    // https://github.com/rust-lang/rust/issues/67939
    unsafe { transmute::<ThreadId, u64>(thread::current().id()) }
    // thread::current().id().as_u64()
}

fn metadata<MetadataName: Into<String>, Name: Into<String>>(
    metadata_name: MetadataName,
    name: Name,
) -> TraceEvent {
    EventBuilder::default()
        .phase(Phase::Metadata)
        .name(metadata_name)
        .argument(Argument::new(name))
        .process_id(process::id())
        .thread_id(current_thread_id())
        .build_for_trace_event()
        .unwrap()
}

impl Profiler {
    pub fn new() -> Profiler {
        Profiler {
            trace_events: Vec::with_capacity(100),
            capture_started_time: Instant::now(),
            current_flow_id: 0,
        }
    }

    pub fn clear(&mut self) {
        self.trace_events.clear();
    }

    pub fn push(&mut self, event: TraceEvent) {
        self.trace_events.push(event);
    }

    pub fn extend(&mut self, profiler: Profiler) {
        self.trace_events.extend(profiler.trace_events);
    }

    fn current_timestamp(&self) -> u64 {
        let micro_seconds = self.capture_started_time.elapsed().as_micros();
        TryFrom::try_from(micro_seconds).unwrap()
    }

    pub fn begin_duration<Name: Into<String>>(&mut self, name: Name) {
        let event = EventBuilder::default()
            .phase(Phase::DurationBegin)
            .name(name)
            .timestamp(self.current_timestamp())
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();

        self.push(event);
    }

    pub fn end_duration(&mut self) {
        let event = EventBuilder::default()
            .phase(Phase::DurationEnd)
            .timestamp(self.current_timestamp())
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();

        self.push(event);
    }

    pub fn begin_and_end_duration<Name: Into<String>, T, Return>(
        &mut self,
        name: Name,
        calculation: T,
    ) -> Return
    where
        T: Fn() -> Return,
    {
        let begin = self.current_timestamp();
        let u = calculation();
        let end = self.current_timestamp();

        let event = EventBuilder::default()
            .phase(Phase::Complete)
            .name(name)
            .timestamp(begin)
            .duration(end - begin)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        self.push(event);

        u
    }

    pub fn current_thread_name<Name: Into<String>>(&mut self, name: Name) {
        self.push(metadata("thread_name", name));
    }

    pub fn current_process_name<Name: Into<String>>(&mut self, name: Name) {
        self.push(metadata("process_name", name));
    }

    pub fn begin_flow<Name: Into<String>, Category: Into<String>>(
        &mut self,
        name: Name,
        category: Category,
    ) -> u64 {
        let id = self.current_flow_id;

        let event = EventBuilder::default()
            .phase(Phase::DurationBegin)
            .name(name)
            .timestamp(self.current_timestamp())
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        self.push(event);

        let end = self.current_timestamp();
        let event = EventBuilder::default()
            .phase(Phase::FlowBegin)
            .name(id.to_string())
            .timestamp(end)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .id(id)
            .category(category)
            .build_for_trace_event()
            .unwrap();
        self.push(event);

        let event = EventBuilder::default()
            .phase(Phase::DurationEnd)
            .timestamp(end)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        self.push(event);

        self.current_flow_id += 1;
        id
    }

    pub fn end_flow<Name: Into<String>, Category: Into<String>>(
        &mut self,
        name: Name,
        category: Category,
        id: u64,
    ) {
        let begin = self.current_timestamp();
        let event = EventBuilder::default()
            .phase(Phase::FlowEnd)
            .name(id.to_string())
            .timestamp(begin)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .id(id)
            .category(category)
            .build_for_trace_event()
            .unwrap();
        self.push(event);

        let event = EventBuilder::default()
            .phase(Phase::DurationBegin)
            .name(name)
            .timestamp(begin)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        self.push(event);

        let event = EventBuilder::default()
            .phase(Phase::DurationEnd)
            .timestamp(self.current_timestamp())
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        self.push(event);
    }

    pub fn save_file<AllocatedPath: AsRef<Path>>(&self, path: AllocatedPath) {
        let memory = TraceEventFormat::new(self.trace_events.clone());
        let serialized = serde_json::to_string(&memory).unwrap();

        let mut file = File::create(path).unwrap();
        let _ = write!(file, "{}", serialized);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_dummy_task() {
        let mut profiler = Profiler::new();
        for _ in 0..10 {
            profiler.end_duration();
        }
    }

    #[test]
    fn simple() {
        let mut profiler = Profiler::new();
        profiler.begin_duration("Task A");
        {
            profiler.begin_and_end_duration("Task B", run_dummy_task);
            profiler.begin_and_end_duration("Task C", run_dummy_task);
        }
        profiler.end_duration();

        profiler.save_file("simple.json");
    }

    #[test]
    fn thread() {
        use std::sync::{mpsc::channel, Arc, Mutex};
        let profiler = Arc::new(Mutex::new(Profiler::new()));

        profiler.lock().unwrap().begin_duration("Task A");

        let (sender, receiver) = channel();
        for name in vec!["Task B", "Task C"] {
            let sender = sender.clone();
            let profiler = profiler.clone();
            thread::spawn(move || {
                let result = profiler
                    .lock()
                    .unwrap()
                    .begin_and_end_duration(name, run_dummy_task);
                sender.send(result).unwrap();
            });
        }

        let _ = receiver.recv();
        let _ = receiver.recv();

        {
            let mut profiler = match profiler.lock() {
                Ok(p) => p,
                Err(e) => e.into_inner(),
            };

            profiler.end_duration();
            profiler.save_file("thread.json");
        }
    }

    #[test]
    fn metadata() {
        let mut profiler = Profiler::new();
        profiler.begin_and_end_duration("Task A", || {});
        // need to rename at the end.
        profiler.current_process_name("metadata test");
        profiler.current_thread_name("metadata thread");
        profiler.save_file("metadata.json");
    }

    #[test]
    fn flow() {
        use std::sync::{mpsc::channel, Arc, Mutex};
        let sync_profiler = Arc::new(Mutex::new(Profiler::new()));

        let main_thread_profiler = Profiler::new();
        let mut mtp = main_thread_profiler;

        let (sender, receiver) = channel();
        for task_name in vec!["Task A", "Task B"] {
            let flow_id = mtp.begin_flow("Spawn", "main_to_spawn");

            let sender = sender.clone();
            let sync_profiler = sync_profiler.clone();
            let profiler = mtp.clone();

            thread::spawn(move || {
                let mut profiler = profiler;
                profiler.clear();

                profiler.end_flow("Begin", "main_to_spawn", flow_id);
                profiler.begin_and_end_duration(task_name, run_dummy_task);

                let flow_id = profiler.begin_flow("End", "spwan_to_main");

                sender.send(flow_id).unwrap();
                sync_profiler.lock().unwrap().extend(profiler);
            });
        }

        for _ in 0..2 {
            let flow_id =
                mtp.begin_and_end_duration("Wait for response", || receiver.recv().unwrap());

            mtp.end_flow("End task", "spwan_to_main", flow_id);
        }

        let sync_profiler = sync_profiler.clone();
        sync_profiler.lock().unwrap().extend(mtp);
        sync_profiler.lock().unwrap().save_file("flow.json");
    }
}
