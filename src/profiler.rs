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
pub struct FlowId {
    id: u64,
    category: String,
}

fn current_thread_id() -> u64 {
    // error[E0658]: use of unstable library feature 'thread_id_value'
    // https://github.com/rust-lang/rust/issues/67939
    unsafe { transmute::<ThreadId, u64>(thread::current().id()) }
    // thread::current().id().as_u64()
}

pub trait Eventalize{
    fn current_timestamp(&self) -> u64;
    fn begin_duration<Name: Into<String>>(&self, name: Name) -> TraceEvent;
    fn end_duration(&self) -> TraceEvent;

    fn begin_and_end_duration<Name: Into<String>, Calculation, Return>(
        &self,
        name: Name,
        calculation: Calculation,
    ) -> (TraceEvent, Return)
    where
        Calculation: Fn() -> Return;

    fn begin_flow(&self, flow_id: FlowId) -> Vec<TraceEvent>;
    fn end_flow(&self, flow_id: FlowId) -> Vec<TraceEvent>;
}

pub fn metadata<MetadataName: Into<String>, Name: Into<String>>(
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

pub fn current_thread_name<Name: Into<String>>(name: Name) -> TraceEvent {
    metadata("thread_name", name)
}

pub fn current_process_name<Name: Into<String>>(name: Name) -> TraceEvent {
    metadata("process_name", name)
}

impl Eventalize for Instant {
    fn current_timestamp(&self) -> u64 {
        let micro_seconds = self.elapsed().as_micros();
        TryFrom::try_from(micro_seconds).unwrap()
    }

    fn begin_duration<Name: Into<String>>(&self, name: Name) -> TraceEvent {
        EventBuilder::default()
            .phase(Phase::DurationBegin)
            .name(name)
            .timestamp(self.current_timestamp())
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap()
    }

    fn end_duration(&self) -> TraceEvent {
        EventBuilder::default()
            .phase(Phase::DurationEnd)
            .timestamp(self.current_timestamp())
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap()
    }

    fn begin_and_end_duration<Name: Into<String>, Calculation, Return>(
        &self,
        name: Name,
        calculation: Calculation,
    ) -> (TraceEvent, Return)
    where
        Calculation: Fn() -> Return,
    {
        let begin = self.current_timestamp();
        let return_value = calculation();
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

        (event, return_value)
    }

    fn begin_flow(&self, flow_id: FlowId) -> Vec<TraceEvent> {
        let mut events = Vec::with_capacity(3);
        let event = EventBuilder::default()
            .phase(Phase::DurationBegin)
            .name("Begin")
            .timestamp(self.current_timestamp())
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        events.push(event);

        let end = self.current_timestamp();
        let event = EventBuilder::default()
            .phase(Phase::FlowBegin)
            .name(flow_id.id.to_string())
            .timestamp(end)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .id(flow_id.id)
            .category(flow_id.category)
            .build_for_trace_event()
            .unwrap();
        events.push(event);

        let event = EventBuilder::default()
            .phase(Phase::DurationEnd)
            .timestamp(end)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        events.push(event);

        events
    }

    fn end_flow(&self, flow_id: FlowId) -> Vec<TraceEvent> {
        let begin = self.current_timestamp();
        let mut events = Vec::with_capacity(3);
        let event = EventBuilder::default()
            .phase(Phase::FlowEnd)
            .name(flow_id.id.to_string())
            .timestamp(begin)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .id(flow_id.id)
            .category(flow_id.category)
            .build_for_trace_event()
            .unwrap();
        events.push(event);

        let event = EventBuilder::default()
            .phase(Phase::DurationBegin)
            .name("End")
            .timestamp(begin)
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        events.push(event);

        let event = EventBuilder::default()
            .phase(Phase::DurationEnd)
            .timestamp(self.current_timestamp())
            .process_id(process::id())
            .thread_id(current_thread_id())
            .build_for_trace_event()
            .unwrap();
        events.push(event);

        events
    }
}

pub fn save_file<AllocatedPath: AsRef<Path>>(path: AllocatedPath, trace_events: Vec<TraceEvent>) {
    let memory = TraceEventFormat::new(trace_events);
    let serialized = serde_json::to_string(&memory).unwrap();

    let mut file = File::create(path).unwrap();
    let _ = write!(file, "{}", serialized);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_dummy_task() {
        let eventalizer = Instant::now();
        for _ in 0..10 {
            eventalizer.end_duration();
        }
    }

    #[test]
    fn simple() {
        let eventalizer = Instant::now();
        let events = vec![
            eventalizer.begin_duration("Task A"),
            match eventalizer.begin_and_end_duration("Task B", run_dummy_task) { (e, _) => e },
            match eventalizer.begin_and_end_duration("Task C", run_dummy_task) { (e, _) => e },
            eventalizer.end_duration(),
        ];
        save_file("simple.json", events);
    }

    #[test]
    fn metadata() {
        let eventalizer = Instant::now();
        let events = vec![
            match eventalizer.begin_and_end_duration("Task A", ||{}) { (e, _) => e },
            // need to rename at the end.
            current_process_name("metadata test"),
            current_thread_name("metadata thread"),
        ];
        save_file("metadata.json", events);
    }

    use std::sync::mpsc::{ channel, Sender, Receiver };

    #[test]
    fn thread() {
        let eventalizer = Instant::now();
        let mut events = vec![
            eventalizer.begin_duration("Task A"),
        ];

        let (sender, receiver) = channel();
        for task_name in vec!["Task B", "Task C"] {
            let sender = sender.clone();
            let eventalizer = eventalizer.clone();

            thread::spawn(move || {
                let events = vec![
                    match eventalizer.begin_and_end_duration(task_name, run_dummy_task) { (e, _) => e },
                ];
                sender.send(events).unwrap();
            });
        }
        events.extend(receiver.recv().unwrap());
        events.extend(receiver.recv().unwrap());

        events.push(eventalizer.end_duration());
        save_file("thread.json", events);
    }

    #[test]
    fn flow() {
        let mut flow_id_counter = 0;
        let eventalizer = Instant::now();
        let mut trace_events = Vec::new();

        let (sender, receiver) = channel();

        for task_name in vec!["Task B", "Task C"] {
            let link_flow_id = FlowId{ id: flow_id_counter, category: "main_to_spawn".to_string() };
            flow_id_counter += 1;
            let unlink_flow_id = FlowId{ id: flow_id_counter, category: "spawn_to_main".to_string() };
            flow_id_counter += 1;

            let events = eventalizer.begin_flow(link_flow_id.clone());
            trace_events.extend(events);

            let sender = sender.clone();
            let eventalizer = eventalizer.clone();

            thread::spawn(move || {
                let mut trace_events = Vec::new();
                trace_events.extend(eventalizer.end_flow(link_flow_id));

                trace_events.push(match eventalizer.begin_and_end_duration(task_name, run_dummy_task) { (e, _) => e });

                trace_events.extend(eventalizer.begin_flow(unlink_flow_id.clone()));
                let package = Package{ trace_events, unlink_flow_id };
                sender.send(package).unwrap();
            });
        }

        for _ in 0..2 {
            let (begin_and_end_event, package) =
                eventalizer.begin_and_end_duration("Wait for response", || receiver.recv().unwrap());
            trace_events.push(begin_and_end_event);
            trace_events.extend(package.trace_events);
            trace_events.extend(eventalizer.end_flow(package.unlink_flow_id));
        }
        save_file("flow.json", trace_events);
    }

    struct Profiler {
        eventalizer: Instant,
        trace_events: Vec<TraceEvent>,
        sender: Sender<Package>,
        receiver: Receiver<Package>,
        flow_id_counter: u64,
        thread_profiler_counter: u64,
    }

    impl Profiler {
        fn new() -> Profiler {
            let (sender, receiver) = channel();
            Profiler{
                eventalizer: Instant::now(),
                trace_events: Vec::new(),
                sender,
                receiver,
                flow_id_counter: 0,
                thread_profiler_counter: 0,
            }
        }

        fn thread_profiler(&mut self) -> ThreadProfiler {
            self.thread_profiler_counter += 1;

            let link_flow_id = FlowId{ id: self.flow_id_counter, category: "main_to_spawn".to_string() };
            self.flow_id_counter += 1;
            let events = self.eventalizer.begin_flow(link_flow_id.clone());
            self.trace_events.extend(events);

            let unlink_flow_id = FlowId{ id: self.flow_id_counter, category: "spwan_to_main".to_string() };
            self.flow_id_counter += 1;

            ThreadProfiler {
                eventalizer: self.eventalizer.clone(),
                sender: self.sender.clone(),
                link_flow_id,
                unlink_flow_id,
            }
        }

        fn wait_for_thread_profiler(&mut self) {
            for _ in 0..self.thread_profiler_counter {
                let package = self.receiver.recv().unwrap();
                self.trace_events.extend(package.trace_events);

                let events = self.eventalizer.end_flow(package.unlink_flow_id);
                self.trace_events.extend(events);
            }
        }
    }

    struct Package {
        trace_events: Vec<TraceEvent>,
        unlink_flow_id: FlowId,
    }

    struct ThreadProfiler {
        eventalizer: Instant,
        sender: Sender<Package>,
        link_flow_id: FlowId,
        unlink_flow_id: FlowId,
    }

    impl ThreadProfiler {
        fn link_thread(&self) -> Package {
            Package {
                trace_events: self.eventalizer.end_flow(self.link_flow_id.clone()),
                unlink_flow_id: self.unlink_flow_id.clone(),
            }
        }

        fn unlink_thread(&self, package: Package) {
            let events = self.eventalizer.begin_flow(package.unlink_flow_id.clone());

            let mut trace_events = package.trace_events.clone();
            trace_events.extend(events);

            let package = Package {
                trace_events,
                unlink_flow_id: package.unlink_flow_id,
            };
            self.sender.send(package).unwrap();
        }
    }

    #[test]
    fn flow2() {
        let mut profiler = Profiler::new();

        for task_name in vec!["Task B", "Task C"] {
            let thread_profiler = profiler.thread_profiler();

            thread::spawn(move || {
                let mut package = thread_profiler.link_thread();

                let (event, _) = thread_profiler.eventalizer.begin_and_end_duration(task_name, || {
                    run_dummy_task();
                });
                package.trace_events.push(event);

                thread_profiler.unlink_thread(package);
            });
        }
        profiler.wait_for_thread_profiler();
        save_file("flow2.json", profiler.trace_events);
    }
}
