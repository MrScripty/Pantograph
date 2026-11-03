//! Async lifetime of bounded service opportunities. No execution or cleanup authority.
use std::sync::{Arc, Mutex};

use crate::scheduler::service_opportunities::{
    ServiceOpportunityGrant, ServiceOpportunityKey, ServiceOpportunityQueue,
};

use super::WorkflowServiceError;

#[derive(Default)]
pub(super) struct SerialServiceOpportunities {
    queue: Mutex<ServiceOpportunityQueue>,
    changed: tokio::sync::Notify,
}

pub(super) struct SerialServiceTurn {
    owner: Arc<SerialServiceOpportunities>,
    ticket: u64,
    grant: Option<ServiceOpportunityGrant>,
}

impl SerialServiceOpportunities {
    pub(super) async fn enter(
        self: &Arc<Self>,
        key: ServiceOpportunityKey,
        priority: i32,
    ) -> Result<SerialServiceTurn, WorkflowServiceError> {
        let ticket = self
            .queue
            .lock()
            .map_err(poisoned)?
            .register(key, priority)?;
        // Construct before suspension: cancellation cannot strand a registration.
        let mut turn = SerialServiceTurn {
            owner: self.clone(),
            ticket,
            grant: None,
        };
        self.changed.notify_waiters();
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            // Register before observing the queue, including notify_waiters races.
            changed.as_mut().enable();
            let grant = self.queue.lock().map_err(poisoned)?.try_grant(ticket)?;
            if let Some(grant) = grant {
                turn.grant = Some(grant);
                return Ok(turn);
            }
            changed.await;
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn pending_len(&self) -> Result<usize, WorkflowServiceError> {
        Ok(self.queue.lock().map_err(poisoned)?.pending_len())
    }
}

impl SerialServiceTurn {
    pub(super) fn grant(&self) -> Option<ServiceOpportunityGrant> {
        self.grant
    }
}

impl Drop for SerialServiceTurn {
    fn drop(&mut self) {
        if let Ok(mut queue) = self.owner.queue.lock() {
            if self.grant.is_some() {
                queue.release();
            } else {
                queue.remove(self.ticket);
            }
        }
        // A poisoned queue refuses future entries; it is never reconstructed.
        self.owner.changed.notify_waiters();
    }
}

fn poisoned<T>(_: std::sync::PoisonError<T>) -> WorkflowServiceError {
    WorkflowServiceError::Internal("serial service opportunity queue poisoned".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::poll;
    use std::task::Poll;

    fn key(client: &str) -> ServiceOpportunityKey {
        ServiceOpportunityKey {
            client_id: client.into(),
            bucket_id: "default".into(),
        }
    }

    #[tokio::test]
    async fn pending_drop_removes_registration_without_grant_or_lost_wakeup() {
        let owner = Arc::new(SerialServiceOpportunities::default());
        let active = owner.enter(key("active"), 0).await.unwrap();
        {
            let waiting = owner.enter(key("cancelled"), i32::MAX);
            tokio::pin!(waiting);
            assert!(poll!(waiting.as_mut()).is_pending());
            assert_eq!(owner.pending_len().unwrap(), 1);
        }
        assert_eq!(owner.pending_len().unwrap(), 0);
        let next = owner.enter(key("next"), i32::MIN);
        tokio::pin!(next);
        assert!(poll!(next.as_mut()).is_pending());
        // Notify arrives before the waiter is polled again.
        drop(active);
        let next = tokio::time::timeout(std::time::Duration::from_secs(1), next)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(owner.pending_len().unwrap(), 0);
        drop(next);
        assert!(owner.enter(key("later"), 0).await.is_ok());
    }

    #[tokio::test]
    async fn selected_turn_remains_exclusive_and_priority_does_not_depend_on_poll_order() {
        let owner = Arc::new(SerialServiceOpportunities::default());
        let active = owner.enter(key("active"), 0).await.unwrap();
        let low = owner.enter(key("low"), -10);
        let high = owner.enter(key("high"), 10);
        tokio::pin!(low, high);
        assert!(poll!(low.as_mut()).is_pending());
        assert!(poll!(high.as_mut()).is_pending());
        drop(active);
        assert!(poll!(low.as_mut()).is_pending());
        let Poll::Ready(Ok(high_turn)) = poll!(high.as_mut()) else {
            panic!("high priority grant")
        };
        assert!(poll!(low.as_mut()).is_pending());
        drop(high_turn);
        let low_turn = low.await.unwrap();
        drop(low_turn);
    }
}
