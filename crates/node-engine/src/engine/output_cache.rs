use std::collections::HashMap;

use super::CachedOutput;
use crate::error::Result;
use crate::types::NodeId;

pub(super) fn resolve_fresh_cached_output(
    cache: &HashMap<NodeId, CachedOutput>,
    node_id: &NodeId,
    input_version: u64,
) -> Result<Option<HashMap<String, serde_json::Value>>> {
    let Some(cached) = cache.get(node_id) else {
        return Ok(None);
    };

    if cached.version != input_version {
        log::debug!(
            "Cache miss for node '{}': version {} != {}",
            node_id,
            cached.version,
            input_version
        );
        return Ok(None);
    }

    log::debug!(
        "Cache hit for node '{}' (version {})",
        node_id,
        input_version
    );
    let outputs = serde_json::from_value(cached.value.clone())?;
    Ok(Some(outputs))
}

pub(super) fn store_completed_output(
    cache: &mut HashMap<NodeId, CachedOutput>,
    versions: &mut HashMap<NodeId, u64>,
    global_version: &mut u64,
    node_id: &NodeId,
    input_version: u64,
    outputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    cache.insert(
        node_id.clone(),
        CachedOutput {
            version: input_version,
            value: serde_json::to_value(outputs)?,
        },
    );

    *global_version += 1;
    versions.insert(node_id.clone(), *global_version);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_fresh_cached_output_returns_outputs_for_matching_version() {
        let cache = HashMap::from([(
            "node-a".to_string(),
            CachedOutput {
                version: 7,
                value: serde_json::json!({
                    "out": "hello"
                }),
            },
        )]);

        let outputs =
            resolve_fresh_cached_output(&cache, &"node-a".to_string(), 7).expect("cache read");

        assert_eq!(
            outputs,
            Some(HashMap::from([(
                "out".to_string(),
                serde_json::json!("hello")
            )]))
        );
    }

    #[test]
    fn resolve_fresh_cached_output_returns_none_for_stale_version() {
        let cache = HashMap::from([(
            "node-a".to_string(),
            CachedOutput {
                version: 3,
                value: serde_json::json!({
                    "out": "hello"
                }),
            },
        )]);

        let outputs =
            resolve_fresh_cached_output(&cache, &"node-a".to_string(), 4).expect("cache read");

        assert_eq!(outputs, None);
    }

    #[test]
    fn store_completed_output_updates_cache_and_versions() {
        let mut cache = HashMap::new();
        let mut versions = HashMap::new();
        let mut global_version = 2;

        store_completed_output(
            &mut cache,
            &mut versions,
            &mut global_version,
            &"node-a".to_string(),
            11,
            &HashMap::from([("out".to_string(), serde_json::json!("value"))]),
        )
        .expect("store cache");

        assert_eq!(global_version, 3);
        assert_eq!(versions.get("node-a"), Some(&3));
        assert_eq!(cache.get("node-a").map(|entry| entry.version), Some(11));
        assert_eq!(
            cache.get("node-a").map(|entry| entry.value.clone()),
            Some(serde_json::json!({ "out": "value" }))
        );
    }
}
