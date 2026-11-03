//! Capacity and final-owner lifetime of original completed embedding results.
//! This domain excludes allocator overhead and all allocations outside that owner.

use super::EmbeddingResult;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
struct Counters {
    registered_owners: u64,
    released_owners: u64,
    live_payload_capacity_bytes: u64,
    peak_observed_live_payload_capacity_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum UnavailableReason {
    CapacityUnavailable,
    CounterOverflow,
    ByteOverflow,
    InvalidRelease,
    LockUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Observation {
    Available(Counters),
    Unavailable { reason: UnavailableReason },
}

impl Observation {
    fn unavailable(&mut self, reason: UnavailableReason) {
        *self = Self::Unavailable { reason };
    }
}

pub(super) struct ResultPayloadLedger {
    scope: Uuid,
    observation: Mutex<Observation>,
}

/// One lease belongs to the immutable original result owner, never its readers.
/// The owner must declare its result before this lease so payload drops first.
pub(super) struct ResultPayloadLease {
    ledger: Arc<ResultPayloadLedger>,
    bytes: u64,
}

impl ResultPayloadLedger {
    pub(super) fn new() -> Self {
        Self {
            scope: Uuid::new_v4(),
            observation: Mutex::new(Observation::Available(Counters::default())),
        }
    }

    pub(super) fn retain(self: &Arc<Self>, bytes: Option<u64>) -> Option<ResultPayloadLease> {
        let mut observation = self.observation.lock().ok()?;
        let Observation::Available(current) = *observation else {
            return None;
        };
        let Some(bytes) = bytes else {
            observation.unavailable(UnavailableReason::CapacityUnavailable);
            return None;
        };
        let Some(registered_owners) = current.registered_owners.checked_add(1) else {
            observation.unavailable(UnavailableReason::CounterOverflow);
            return None;
        };
        let Some(live_payload_capacity_bytes) =
            current.live_payload_capacity_bytes.checked_add(bytes)
        else {
            observation.unavailable(UnavailableReason::ByteOverflow);
            return None;
        };
        *observation = Observation::Available(Counters {
            registered_owners,
            live_payload_capacity_bytes,
            peak_observed_live_payload_capacity_bytes: current
                .peak_observed_live_payload_capacity_bytes
                .max(live_payload_capacity_bytes),
            ..current
        });
        Some(ResultPayloadLease {
            ledger: Arc::clone(self),
            bytes,
        })
    }

    fn release(&self, bytes: u64) {
        let Ok(mut observation) = self.observation.lock() else {
            return;
        };
        let Observation::Available(current) = *observation else {
            return;
        };
        if current.released_owners >= current.registered_owners {
            observation.unavailable(UnavailableReason::InvalidRelease);
            return;
        }
        let Some(live_payload_capacity_bytes) =
            current.live_payload_capacity_bytes.checked_sub(bytes)
        else {
            observation.unavailable(UnavailableReason::InvalidRelease);
            return;
        };
        let Some(released_owners) = current.released_owners.checked_add(1) else {
            observation.unavailable(UnavailableReason::CounterOverflow);
            return;
        };
        *observation = Observation::Available(Counters {
            released_owners,
            live_payload_capacity_bytes,
            ..current
        });
    }

    pub(super) fn diagnostic(&self, phase: &'static str) -> serde_json::Value {
        let observation = self.observation.lock().map_or(
            Observation::Unavailable {
                reason: UnavailableReason::LockUnavailable,
            },
            |observation| *observation,
        );
        // Fixed scalars are copied before JSON allocation or logging begins.
        serde_json::json!({
            "phase": phase,
            "scope": self.scope.to_string(),
            "source": "observed_owned_vec_capacity",
            "domain": "original_completed_result_payload",
            "units": "bytes",
            "observation": observation,
            "exclusions": [
                "construction_transients", "allocator_overhead", "caller_clones",
                "inputs_and_tokenizer", "candle_intermediates_and_aliases",
                "scratch_and_rayon", "residency"
            ],
        })
    }

    pub(super) fn log(&self, phase: &'static str) {
        log::info!(target: "inference::candle_result_payload", "{}", self.diagnostic(phase));
    }
}

impl Drop for ResultPayloadLease {
    fn drop(&mut self) {
        // No logger, provider, callback or authority transition runs from Drop.
        self.ledger.release(self.bytes);
    }
}

fn capacity_bytes(capacity: usize, element_size: usize) -> Option<u64> {
    u64::try_from(capacity)
        .ok()?
        .checked_mul(u64::try_from(element_size).ok()?)
}

// Vec capacity, rather than a slice length, is the observed ownership quantity.
pub(super) fn embedding_payload_capacity(payload: &Vec<EmbeddingResult>) -> Option<u64> {
    let mut bytes = capacity_bytes(payload.capacity(), std::mem::size_of::<EmbeddingResult>())?;
    for result in payload {
        bytes = bytes.checked_add(capacity_bytes(
            result.vector.capacity(),
            std::mem::size_of::<f32>(),
        )?)?;
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counters(ledger: &ResultPayloadLedger) -> Counters {
        match *ledger.observation.lock().unwrap() {
            Observation::Available(counters) => counters,
            observation => panic!("expected available counters, got {observation:?}"),
        }
    }

    fn assert_unavailable(ledger: &Arc<ResultPayloadLedger>, reason: UnavailableReason) {
        assert_eq!(
            *ledger.observation.lock().unwrap(),
            Observation::Unavailable { reason }
        );
        assert!(ledger.retain(Some(0)).is_none());
        ledger.release(0);
        assert_eq!(
            *ledger.observation.lock().unwrap(),
            Observation::Unavailable { reason }
        );
        let diagnostic = ledger.diagnostic("completed");
        assert!(diagnostic["observation"]
            .get("live_payload_capacity_bytes")
            .is_none());
    }

    #[test]
    fn payload_capacity_counts_outer_spare_slots_and_initialized_inner_capacities() {
        assert_eq!(embedding_payload_capacity(&Vec::new()), Some(0));
        let empty = Vec::with_capacity(3);
        assert_eq!(
            embedding_payload_capacity(&empty),
            Some((empty.capacity() * std::mem::size_of::<EmbeddingResult>()) as u64)
        );
        let mut payload = Vec::with_capacity(5);
        let mut first = Vec::with_capacity(7);
        first.push(1.25);
        payload.push(EmbeddingResult {
            vector: first,
            token_count: 12345,
        });
        payload.push(EmbeddingResult {
            vector: Vec::with_capacity(11),
            token_count: 0,
        });
        let expected = payload.capacity() * std::mem::size_of::<EmbeddingResult>()
            + payload
                .iter()
                .map(|result| result.vector.capacity() * 4)
                .sum::<usize>();
        assert_eq!(embedding_payload_capacity(&payload), Some(expected as u64));
        assert_eq!(
            capacity_bytes(usize::MAX, usize::MAX),
            (usize::MAX as u64).checked_mul(usize::MAX as u64)
        );
    }

    #[test]
    fn leases_track_overlapping_original_owners_and_accept_zero_capacity() {
        let ledger = Arc::new(ResultPayloadLedger::new());
        let first = ledger.retain(Some(17)).unwrap();
        let second = ledger.retain(Some(23)).unwrap();
        let empty = ledger.retain(Some(0)).unwrap();
        assert_eq!(
            counters(&ledger),
            Counters {
                registered_owners: 3,
                released_owners: 0,
                live_payload_capacity_bytes: 40,
                peak_observed_live_payload_capacity_bytes: 40,
            }
        );
        drop(first);
        assert_eq!(counters(&ledger).live_payload_capacity_bytes, 23);
        drop(second);
        drop(empty);
        assert_eq!(
            counters(&ledger),
            Counters {
                registered_owners: 3,
                released_owners: 3,
                live_payload_capacity_bytes: 0,
                peak_observed_live_payload_capacity_bytes: 40,
            }
        );
    }

    #[test]
    fn unavailable_capacity_overflow_and_invalid_releases_are_absorbing() {
        let missing = Arc::new(ResultPayloadLedger::new());
        assert!(missing.retain(None).is_none());
        assert_unavailable(&missing, UnavailableReason::CapacityUnavailable);

        let count = Arc::new(ResultPayloadLedger::new());
        *count.observation.lock().unwrap() = Observation::Available(Counters {
            registered_owners: u64::MAX,
            ..Counters::default()
        });
        assert!(count.retain(Some(0)).is_none());
        assert_unavailable(&count, UnavailableReason::CounterOverflow);

        let bytes = Arc::new(ResultPayloadLedger::new());
        let lease = bytes.retain(Some(u64::MAX)).unwrap();
        assert!(bytes.retain(Some(1)).is_none());
        drop(lease);
        assert_unavailable(&bytes, UnavailableReason::ByteOverflow);

        let unmatched = Arc::new(ResultPayloadLedger::new());
        unmatched.release(0);
        assert_unavailable(&unmatched, UnavailableReason::InvalidRelease);

        let underflow = Arc::new(ResultPayloadLedger::new());
        let lease = underflow.retain(Some(1)).unwrap();
        underflow.release(2);
        drop(lease);
        assert_unavailable(&underflow, UnavailableReason::InvalidRelease);
    }

    #[test]
    fn poison_remains_unavailable_without_recovering_or_panicking_on_release() {
        let ledger = Arc::new(ResultPayloadLedger::new());
        let lease = ledger.retain(Some(7)).unwrap();
        let poison = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = ledger.observation.lock().unwrap();
            panic!("controlled ledger poison");
        }));
        assert!(poison.is_err());
        drop(lease);
        assert!(ledger.retain(Some(0)).is_none());
        for phase in ["completed", "drain"] {
            assert_eq!(
                ledger.diagnostic(phase)["observation"],
                serde_json::json!({
                    "state": "unavailable", "reason": "lock_unavailable"
                })
            );
        }
        assert!(ledger.observation.is_poisoned());
    }

    #[test]
    fn diagnostics_are_fixed_scalar_summaries_with_an_opaque_scope() {
        let ledger = Arc::new(ResultPayloadLedger::new());
        let other = ResultPayloadLedger::new();
        let lease = ledger.retain(Some(42)).unwrap();
        let diagnostic = ledger.diagnostic("completed");
        assert_eq!(diagnostic.as_object().unwrap().len(), 7);
        assert_eq!(diagnostic["source"], "observed_owned_vec_capacity");
        assert_eq!(diagnostic["domain"], "original_completed_result_payload");
        assert_eq!(diagnostic["units"], "bytes");
        assert_eq!(diagnostic["observation"].as_object().unwrap().len(), 5);
        assert_eq!(diagnostic["exclusions"].as_array().unwrap().len(), 7);
        let scope = diagnostic["scope"].as_str().unwrap();
        assert_eq!(Uuid::parse_str(scope).unwrap(), ledger.scope);
        assert_ne!(ledger.scope, other.scope);
        assert_eq!(diagnostic["scope"], ledger.diagnostic("drain")["scope"]);
        assert!(diagnostic.to_string().len() < 2048);
        assert!(diagnostic.get("vector").is_none());
        assert!(diagnostic.get("token_count").is_none());
        assert!(diagnostic.get("attempt_id").is_none());
        drop(lease);
    }
}
