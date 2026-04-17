use super::{
    BroadcastEventSink, CallbackEventSink, CompositeEventSink, EventSink, NullEventSink,
    VecEventSink, WorkflowEvent,
};

#[test]
fn test_vec_event_sink() {
    let sink = VecEventSink::new();

    sink.send(WorkflowEvent::task_progress(
        "task1",
        "exec1",
        0.5,
        Some("halfway".to_string()),
    ))
    .unwrap();

    let events = sink.events();
    assert_eq!(events.len(), 1);

    match &events[0] {
        WorkflowEvent::TaskProgress {
            task_id,
            progress,
            occurred_at_ms,
            ..
        } => {
            assert_eq!(task_id, "task1");
            assert_eq!(*progress, 0.5);
            assert!(occurred_at_ms.is_some());
        }
        _ => panic!("Expected TaskProgress event"),
    }
}

#[test]
fn test_null_event_sink() {
    let sink = NullEventSink;
    sink.send(WorkflowEvent::task_progress("task1", "exec1", 1.0, None))
        .unwrap();
}

#[test]
fn test_broadcast_event_sink() {
    let (sink, mut rx) = BroadcastEventSink::new(16);
    let mut rx2 = sink.subscribe();

    sink.send(WorkflowEvent::task_progress("task1", "exec1", 0.5, None))
        .unwrap();

    let event = rx.try_recv().unwrap();
    assert!(matches!(event, WorkflowEvent::TaskProgress { .. }));

    let event2 = rx2.try_recv().unwrap();
    assert!(matches!(event2, WorkflowEvent::TaskProgress { .. }));
}

#[test]
fn test_broadcast_no_receivers() {
    let (sink, rx) = BroadcastEventSink::new(16);
    drop(rx);

    let result = sink.send(WorkflowEvent::task_progress("task1", "exec1", 1.0, None));
    assert!(result.is_ok());
}

#[test]
fn test_callback_event_sink() {
    let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected_clone = collected.clone();

    let sink = CallbackEventSink::new(move |event| {
        collected_clone.lock().unwrap().push(event);
    });

    sink.send(WorkflowEvent::task_progress("task1", "exec1", 0.5, None))
        .unwrap();
    sink.send(WorkflowEvent::task_progress("task1", "exec1", 1.0, None))
        .unwrap();

    assert_eq!(collected.lock().unwrap().len(), 2);
}

#[test]
fn test_composite_event_sink() {
    let mut composite = CompositeEventSink::new();
    let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected_clone = collected.clone();

    composite.add(Box::new(CallbackEventSink::new(move |event| {
        collected_clone.lock().unwrap().push(event);
    })));
    composite.add(Box::new(NullEventSink));
    assert_eq!(composite.len(), 2);

    composite
        .send(WorkflowEvent::task_progress("task1", "exec1", 0.5, None))
        .unwrap();

    assert_eq!(collected.lock().unwrap().len(), 1);
}

#[test]
fn test_composite_empty() {
    let composite = CompositeEventSink::new();
    assert!(composite.is_empty());

    composite
        .send(WorkflowEvent::task_progress("task1", "exec1", 0.5, None))
        .unwrap();
}
