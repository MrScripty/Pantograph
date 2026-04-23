use super::*;

#[test]
fn test_version_tracking() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    // Initially all versions are 0
    assert_eq!(engine.compute_input_version(&"a".to_string(), &graph), 0);
    assert_eq!(engine.compute_input_version(&"b".to_string(), &graph), 0);

    // Mark 'a' as modified
    engine.mark_modified(&"a".to_string());

    // 'b' input version should change (depends on 'a')
    assert_eq!(engine.compute_input_version(&"b".to_string(), &graph), 1);

    // 'a' input version should still be 0 (no dependencies)
    assert_eq!(engine.compute_input_version(&"a".to_string(), &graph), 0);
}

#[test]
fn test_cache_invalidation() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    // Cache output for 'b'
    engine.cache_output(&"b".to_string(), serde_json::json!("cached_value"), &graph);

    // Should be able to get cached value
    assert!(engine.get_cached(&"b".to_string(), &graph).is_some());

    // Mark 'a' as modified
    engine.mark_modified(&"a".to_string());

    // Cache for 'b' should now be invalid (input version changed)
    assert!(engine.get_cached(&"b".to_string(), &graph).is_none());
}

#[test]
fn test_cache_stats() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
    engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    engine.mark_modified(&"c".to_string());

    let stats = engine.cache_stats();
    assert_eq!(stats.cached_nodes, 2);
    assert_eq!(stats.total_versions, 1); // Only 'c' has been modified
    assert_eq!(stats.global_version, 1);
}

#[test]
fn test_reconcile_isolated_run_merges_changed_state_without_touching_unrelated_entries() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);

    let base = engine.clone();
    let mut isolated = base.clone();
    isolated.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    isolated.mark_modified(&"c".to_string());

    engine.cache_output(&"z".to_string(), serde_json::json!("keep"), &graph);
    engine.reconcile_isolated_run(&base, &isolated);

    assert!(engine.cache.contains_key("a"));
    assert!(engine.cache.contains_key("b"));
    assert!(engine.cache.contains_key("z"));
    assert_eq!(engine.versions.get("c"), Some(&1));
    assert_eq!(engine.global_version, 1);
}

#[test]
fn test_reconcile_isolated_run_removes_entries_cleared_from_base_state() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

    let base = engine.clone();
    let mut isolated = base.clone();
    isolated.invalidate_downstream(&"b".to_string(), &graph);

    engine.reconcile_isolated_run(&base, &isolated);

    assert!(!engine.cache.contains_key("b"));
    assert!(!engine.cache.contains_key("c"));
}

#[test]
fn test_invalidate_downstream() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    // Cache all nodes
    engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
    engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

    assert_eq!(engine.cache_stats().cached_nodes, 3);

    // Invalidate downstream from 'a' (should invalidate a, b, c)
    engine.invalidate_downstream(&"a".to_string(), &graph);

    assert_eq!(engine.cache_stats().cached_nodes, 0);
}

#[test]
fn test_invalidate_downstream_partial() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    // Cache all nodes
    engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
    engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

    // Invalidate downstream from 'b' (should invalidate b, c but not a)
    engine.invalidate_downstream(&"b".to_string(), &graph);

    assert_eq!(engine.cache_stats().cached_nodes, 1);
    assert!(engine.cache.contains_key("a"));
    assert!(!engine.cache.contains_key("b"));
    assert!(!engine.cache.contains_key("c"));
}
