use super::*;

#[test]
fn parse_storage_policy_defaults_to_memory() {
    let inputs = HashMap::new();
    assert!(matches!(
        parse_storage_policy(&inputs),
        StoragePolicy::MemoryOnly
    ));
}

#[test]
fn parse_storage_policy_supports_disk_and_both() {
    let mut disk_inputs = HashMap::new();
    disk_inputs.insert("storage_policy".to_string(), serde_json::json!("disk"));
    assert!(matches!(
        parse_storage_policy(&disk_inputs),
        StoragePolicy::DiskOnly
    ));

    let mut both_inputs = HashMap::new();
    both_inputs.insert("storage_policy".to_string(), serde_json::json!("both"));
    assert!(matches!(
        parse_storage_policy(&both_inputs),
        StoragePolicy::MemoryAndDisk
    ));
}

#[test]
fn parse_markers_returns_empty_when_missing() {
    let inputs = HashMap::new();
    let markers = parse_markers(&inputs).expect("missing markers should default to empty");
    assert!(markers.is_empty());
}

#[test]
fn parse_markers_parses_marker_payloads() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "markers".to_string(),
        serde_json::json!([{
            "name": "system",
            "tokenPosition": 12,
            "description": "prefix boundary"
        }]),
    );

    let markers = parse_markers(&inputs).expect("marker payload should parse");
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].name, "system");
    assert_eq!(markers[0].token_position, 12);
}
