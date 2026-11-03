//! Bounded service opportunities for already validated Ready work.
//!
//! This policy owns ordering only. Its async caller owns registration lifetime,
//! eligibility revalidation, and release after the existing serial scope ends.

use crate::workflow::WorkflowServiceError;

const MAX_PENDING_REGISTRATIONS: usize = 64;
const MAX_PENDING_UNITS: usize = 32;
const PROTECTION_BYPASSES: u8 = 2;

/// Backend-validated attribution; sessions in the same unit share bypass debt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceOpportunityKey {
    pub(crate) client_id: String,
    pub(crate) bucket_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ServiceOpportunityGrant {
    pub(crate) ticket: u64,
    pub(crate) protected: bool,
    /// The selected unit's bypass count before this grant resets it.
    pub(crate) bypasses: u8,
    /// Highest other-unit priority at selection, for bounded qualification only.
    pub(crate) competing_priority: Option<i32>,
}

#[derive(Debug)]
struct PendingMember {
    ticket: u64,
    priority: i32,
}

#[derive(Debug)]
struct PendingUnit {
    key: ServiceOpportunityKey,
    service_order: u64,
    bypasses: u8,
    members: Vec<PendingMember>,
}

/// At most one granted turn, 64 pending members, and 32 pending units.
///
/// Other-unit grants saturate debt at two. Protected units precede ordinary
/// priority, ordered by their last service (or first arrival). Every grant
/// refreshes that order, including when old members remain. Consequently a
/// continuously pending unit waits at most 33 other-unit grants for an
/// opportunity. This promises neither wall time nor fairness among unequal
/// priorities within one unit.
#[derive(Debug, Default)]
pub(crate) struct ServiceOpportunityQueue {
    units: Vec<PendingUnit>,
    last_ticket: u64,
    last_service_order: u64,
    busy: bool,
}

impl ServiceOpportunityQueue {
    pub(crate) fn register(
        &mut self,
        key: ServiceOpportunityKey,
        priority: i32,
    ) -> Result<u64, WorkflowServiceError> {
        if self.pending_len() >= MAX_PENDING_REGISTRATIONS {
            return Err(WorkflowServiceError::InvalidRequest(
                "service opportunity pending registration limit reached".to_string(),
            ));
        }
        let unit_index = self.units.iter().position(|unit| unit.key == key);
        if unit_index.is_none() && self.units.len() >= MAX_PENDING_UNITS {
            return Err(WorkflowServiceError::InvalidRequest(
                "service opportunity pending entitlement unit limit reached".to_string(),
            ));
        }

        // Preflight both identities before adding work or advancing either one.
        let ticket = self.last_ticket.checked_add(1).ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(
                "service opportunity registration identity exhausted".to_string(),
            )
        })?;
        let new_service_order = if unit_index.is_none() {
            Some(self.next_service_order()?)
        } else {
            None
        };
        let member = PendingMember { ticket, priority };
        if let Some(unit_index) = unit_index {
            // Same-unit arrivals preserve both service order and existing debt.
            self.units[unit_index].members.push(member);
        } else if let Some(service_order) = new_service_order {
            self.units.push(PendingUnit {
                key,
                service_order,
                bypasses: 0,
                members: vec![member],
            });
            self.last_service_order = service_order;
        }
        self.last_ticket = ticket;
        Ok(ticket)
    }

    /// Grants only the current winner's ticket; nonwinning/absent tickets wait.
    /// A grant consumes the member and holds the turn until `release`.
    pub(crate) fn try_grant(
        &mut self,
        ticket: u64,
    ) -> Result<Option<ServiceOpportunityGrant>, WorkflowServiceError> {
        if self.busy {
            return Ok(None);
        }
        let Some((unit_index, member_index)) = self.selected_member() else {
            return Ok(None);
        };
        if self.units[unit_index].members[member_index].ticket != ticket {
            return Ok(None);
        }

        // Exhaustion cannot consume a waiter, change debt, or acquire the turn.
        let service_order = self.next_service_order()?;
        let bypasses = self.units[unit_index].bypasses;
        let competing_priority = self
            .units
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != unit_index)
            .flat_map(|(_, unit)| &unit.members)
            .map(|member| member.priority)
            .max();
        self.units[unit_index].members.remove(member_index);
        for (index, unit) in self.units.iter_mut().enumerate() {
            if index == unit_index {
                unit.bypasses = 0;
                unit.service_order = service_order;
            } else {
                unit.bypasses = unit.bypasses.saturating_add(1).min(PROTECTION_BYPASSES);
            }
        }
        if self.units[unit_index].members.is_empty() {
            self.units.remove(unit_index);
        }
        self.last_service_order = service_order;
        self.busy = true;
        Ok(Some(ServiceOpportunityGrant {
            ticket,
            protected: bypasses >= PROTECTION_BYPASSES,
            bypasses,
            competing_priority,
        }))
    }

    /// Pending cancellation changes no other unit's debt or service order.
    pub(crate) fn remove(&mut self, ticket: u64) {
        for unit_index in 0..self.units.len() {
            if let Some(member_index) = self.units[unit_index]
                .members
                .iter()
                .position(|member| member.ticket == ticket)
            {
                self.units[unit_index].members.remove(member_index);
                if self.units[unit_index].members.is_empty() {
                    self.units.remove(unit_index);
                }
                return;
            }
        }
    }

    /// The caller releases only after dropping the existing serial scope.
    /// This changes no physical ownership or cleanup state.
    pub(crate) fn release(&mut self) {
        self.busy = false;
    }

    pub(crate) fn pending_len(&self) -> usize {
        self.units.iter().map(|unit| unit.members.len()).sum()
    }

    fn next_service_order(&self) -> Result<u64, WorkflowServiceError> {
        self.last_service_order.checked_add(1).ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(
                "service opportunity service-order identity exhausted".to_string(),
            )
        })
    }

    fn selected_member(&self) -> Option<(usize, usize)> {
        let protected_unit = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.bypasses >= PROTECTION_BYPASSES)
            .min_by_key(|(_, unit)| unit.service_order)
            .map(|(index, _)| index);
        self.units
            .iter()
            .enumerate()
            .filter(|(index, _)| protected_unit.is_none_or(|protected| protected == *index))
            .flat_map(|(unit_index, unit)| {
                unit.members
                    .iter()
                    .enumerate()
                    .map(move |(member_index, member)| (unit_index, member_index, member))
            })
            .min_by(|(_, _, left), (_, _, right)| {
                right
                    .priority
                    .cmp(&left.priority)
                    .then_with(|| left.ticket.cmp(&right.ticket))
            })
            .map(|(unit_index, member_index, _)| (unit_index, member_index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(bucket_id: &str) -> ServiceOpportunityKey {
        ServiceOpportunityKey {
            client_id: "authenticated-client".to_string(),
            bucket_id: bucket_id.to_string(),
        }
    }

    fn register(queue: &mut ServiceOpportunityQueue, bucket: &str, priority: i32) -> u64 {
        queue.register(key(bucket), priority).unwrap()
    }

    fn grant(queue: &mut ServiceOpportunityQueue, ticket: u64) -> ServiceOpportunityGrant {
        let granted = queue.try_grant(ticket).unwrap().expect("expected winner");
        assert_eq!(granted.ticket, ticket);
        queue.release();
        granted
    }

    fn assert_invalid<T: std::fmt::Debug>(result: Result<T, WorkflowServiceError>, expected: &str) {
        match result {
            Err(WorkflowServiceError::InvalidRequest(message)) => assert_eq!(message, expected),
            other => panic!("expected InvalidRequest({expected:?}), got {other:?}"),
        }
    }

    #[test]
    fn ordinary_grants_follow_priority_then_member_fifo_including_i32_extrema() {
        let mut queue = ServiceOpportunityQueue::default();
        let lowest = register(&mut queue, "same-unit", i32::MIN);
        let first_highest = register(&mut queue, "same-unit", i32::MAX);
        let second_highest = register(&mut queue, "same-unit", i32::MAX);
        let middle = register(&mut queue, "same-unit", 0);

        assert_eq!(queue.try_grant(lowest).unwrap(), None);
        for ticket in [first_highest, second_highest, middle, lowest] {
            assert_eq!(
                grant(&mut queue, ticket),
                ServiceOpportunityGrant {
                    ticket,
                    protected: false,
                    bypasses: 0,
                    competing_priority: None,
                }
            );
        }
        assert_eq!(queue.pending_len(), 0);
        assert!(queue.units.is_empty());
    }

    #[test]
    fn ordinary_equal_priorities_follow_fifo_across_units() {
        let mut queue = ServiceOpportunityQueue::default();
        let a = register(&mut queue, "A", 7);
        let b = register(&mut queue, "B", 7);
        let c = register(&mut queue, "C", 7);

        assert_eq!(queue.try_grant(c).unwrap(), None);
        assert!(!grant(&mut queue, a).protected);
        assert!(!grant(&mut queue, b).protected);
        assert_eq!(grant(&mut queue, c).bypasses, 2);
    }

    #[test]
    fn repeated_highest_foreground_cannot_bypass_lowest_background_more_than_twice() {
        let mut queue = ServiceOpportunityQueue::default();
        let background = register(&mut queue, "B", i32::MIN);
        let foreground_1 = register(&mut queue, "A", i32::MAX);
        assert_eq!(grant(&mut queue, foreground_1).bypasses, 0);
        let foreground_2 = register(&mut queue, "A", i32::MAX);
        assert_eq!(grant(&mut queue, foreground_2).bypasses, 0);
        let foreground_3 = register(&mut queue, "A", i32::MAX);

        assert_eq!(queue.try_grant(foreground_3).unwrap(), None);
        assert_eq!(
            grant(&mut queue, background),
            ServiceOpportunityGrant {
                ticket: background,
                protected: true,
                bypasses: 2,
                competing_priority: Some(i32::MAX),
            }
        );
        assert!(!grant(&mut queue, foreground_3).protected);
    }

    #[test]
    fn same_unit_session_arrivals_preserve_debt_and_protected_member_priority_fifo() {
        let mut queue = ServiceOpportunityQueue::default();
        let original = register(&mut queue, "B", -5);
        let foreground_1 = register(&mut queue, "A", 100);
        grant(&mut queue, foreground_1);
        let first_new_session = register(&mut queue, "B", 1);
        let foreground_2 = register(&mut queue, "A", 100);
        grant(&mut queue, foreground_2);
        let second_new_session = register(&mut queue, "B", 1);
        let foreground_3 = register(&mut queue, "A", 100);

        assert_eq!(queue.units.len(), 2);
        assert_eq!(queue.try_grant(original).unwrap(), None);
        assert_eq!(queue.try_grant(second_new_session).unwrap(), None);
        assert_eq!(queue.try_grant(foreground_3).unwrap(), None);
        assert!(grant(&mut queue, first_new_session).protected);
        grant(&mut queue, foreground_3);
        assert_eq!(grant(&mut queue, second_new_session).bypasses, 1);
        assert_eq!(grant(&mut queue, original).bypasses, 0);
    }

    #[test]
    fn refreshed_service_order_serves_newer_protected_unit_with_old_backlogs() {
        let mut queue = ServiceOpportunityQueue::default();
        let a: Vec<_> = (0..8).map(|_| register(&mut queue, "A", 30)).collect();
        let b: Vec<_> = (0..8).map(|_| register(&mut queue, "B", 20)).collect();
        let c: Vec<_> = (0..8).map(|_| register(&mut queue, "C", 10)).collect();
        grant(&mut queue, a[0]);
        grant(&mut queue, a[1]);
        let d: Vec<_> = (0..8).map(|_| register(&mut queue, "D", 0)).collect();

        // Without refreshing service order, old A/B/C rotate forever before D.
        // All four units retain backlog throughout this hand-written trace.
        for ticket in [b[0], c[0], a[2], d[0], b[1], c[1], a[3], d[1]] {
            assert_eq!(
                grant(&mut queue, ticket),
                ServiceOpportunityGrant {
                    ticket,
                    protected: true,
                    bypasses: 2,
                    competing_priority: Some(if a.contains(&ticket) { 20 } else { 30 }),
                }
            );
        }
        assert_eq!(queue.units.len(), 4);
        assert_eq!(queue.pending_len(), 22);
    }

    #[test]
    fn protected_units_follow_service_order_even_when_newer_unit_has_higher_priority() {
        let mut queue = ServiceOpportunityQueue::default();
        let old = register(&mut queue, "old", i32::MIN);
        let new = register(&mut queue, "new", 10);
        let high_1 = register(&mut queue, "high", i32::MAX);
        let high_2 = register(&mut queue, "high", i32::MAX);
        grant(&mut queue, high_1);
        grant(&mut queue, high_2);

        assert_eq!(queue.try_grant(new).unwrap(), None);
        assert!(grant(&mut queue, old).protected);
        // The intervening old grant saturates, rather than increasing, new debt.
        assert_eq!(grant(&mut queue, new).bypasses, 2);
    }

    #[test]
    fn full_unit_population_preserves_opportunity_within_33_other_grants() {
        let mut queue = ServiceOpportunityQueue::default();
        let high_1 = register(&mut queue, "high", i32::MAX);
        let high_2 = register(&mut queue, "high", i32::MAX);
        let protected_ahead: Vec<_> = (0..30)
            .map(|index| register(&mut queue, &format!("ahead-{index}"), 0))
            .collect();
        let last = register(&mut queue, "last", i32::MIN);
        grant(&mut queue, high_1);
        grant(&mut queue, high_2);
        let new_high = register(&mut queue, "high", i32::MAX);

        for ticket in protected_ahead {
            assert_eq!(queue.try_grant(last).unwrap(), None);
            assert!(grant(&mut queue, ticket).protected);
        }
        assert_eq!(queue.try_grant(new_high).unwrap(), None);
        assert_eq!(grant(&mut queue, last).bypasses, 2);
        // Exactly 32 other grants in this trace, inside the contract's bound 33.
        assert!(grant(&mut queue, new_high).protected);
    }

    #[test]
    fn removing_pending_members_preserves_other_debt_and_cleans_empty_units() {
        let mut queue = ServiceOpportunityQueue::default();
        let background = register(&mut queue, "B", 0);
        let cancelled_session = register(&mut queue, "B", 0);
        let foreground = register(&mut queue, "A", 10);
        let cancelled_unit = register(&mut queue, "C", -10);
        grant(&mut queue, foreground);
        let background_order = queue.units[0].service_order;

        queue.remove(cancelled_session);
        queue.remove(cancelled_unit);
        queue.remove(cancelled_session);
        queue.remove(u64::MAX);
        assert_eq!(queue.pending_len(), 1);
        assert_eq!(queue.units.len(), 1);
        assert_eq!(queue.units[0].service_order, background_order);
        assert_eq!(queue.units[0].bypasses, 1);
        assert_eq!(grant(&mut queue, background).bypasses, 1);
        let returning = register(&mut queue, "B", 0);
        assert_eq!(grant(&mut queue, returning).bypasses, 0);
    }

    #[test]
    fn busy_turn_blocks_all_grants_and_selected_removal_does_not_release_it() {
        let mut queue = ServiceOpportunityQueue::default();
        let a = register(&mut queue, "A", 10);
        let b = register(&mut queue, "B", 0);
        let selected = queue.try_grant(a).unwrap().unwrap();
        assert_eq!(selected.ticket, a);
        assert_eq!(queue.pending_len(), 1);
        let order = queue.last_service_order;

        queue.remove(a);
        assert_eq!(queue.try_grant(b).unwrap(), None);
        assert_eq!(queue.try_grant(a).unwrap(), None);
        assert_eq!(queue.last_service_order, order);
        assert_eq!(queue.units[0].bypasses, 1);
        queue.release();
        assert_eq!(grant(&mut queue, b).bypasses, 1);
        assert_eq!(queue.try_grant(a).unwrap(), None);
    }

    #[test]
    fn registration_cap_refuses_without_eviction_or_identity_consumption() {
        let mut queue = ServiceOpportunityQueue::default();
        let tickets: Vec<_> = (0..64).map(|_| register(&mut queue, "A", 0)).collect();
        let last_ticket = queue.last_ticket;
        let last_service_order = queue.last_service_order;
        assert_invalid(
            queue.register(key("A"), 100),
            "service opportunity pending registration limit reached",
        );
        assert_eq!(queue.pending_len(), 64);
        assert_eq!(queue.last_ticket, last_ticket);
        assert_eq!(queue.last_service_order, last_service_order);
        queue.remove(tickets[0]);
        let replacement = register(&mut queue, "A", 0);
        assert_eq!(replacement, last_ticket + 1);
        for ticket in tickets.into_iter().skip(1).chain([replacement]) {
            grant(&mut queue, ticket);
        }
    }

    #[test]
    fn unit_cap_uses_client_and_bucket_and_allows_existing_unit_members() {
        let mut queue = ServiceOpportunityQueue::default();
        let tickets: Vec<_> = (0..32)
            .map(|index| register(&mut queue, &format!("bucket-{index}"), 0))
            .collect();
        let last_ticket = queue.last_ticket;
        let last_service_order = queue.last_service_order;
        assert_invalid(
            queue.register(
                ServiceOpportunityKey {
                    client_id: "another-authenticated-client".to_string(),
                    bucket_id: "bucket-0".to_string(),
                },
                0,
            ),
            "service opportunity pending entitlement unit limit reached",
        );
        assert_eq!(queue.last_ticket, last_ticket);
        assert_eq!(queue.last_service_order, last_service_order);
        register(&mut queue, "bucket-0", 0);
        assert_eq!(queue.units.len(), 32);
        assert_eq!(queue.pending_len(), 33);
        queue.remove(tickets[1]);
        register(&mut queue, "replacement", 0);
        assert_eq!(queue.units.len(), 32);
    }

    #[test]
    fn exhausted_member_identity_refuses_existing_and_new_units_atomically() {
        let mut queue = ServiceOpportunityQueue::default();
        let existing = register(&mut queue, "A", 0);
        queue.last_ticket = u64::MAX - 1;
        let final_ticket = register(&mut queue, "A", 0);
        assert_eq!(final_ticket, u64::MAX);
        let service_order = queue.last_service_order;
        for bucket in ["A", "B"] {
            assert_invalid(
                queue.register(key(bucket), 0),
                "service opportunity registration identity exhausted",
            );
        }
        assert_eq!(queue.pending_len(), 2);
        assert_eq!(queue.units.len(), 1);
        assert_eq!(queue.last_service_order, service_order);
        grant(&mut queue, existing);
        grant(&mut queue, final_ticket);
    }

    #[test]
    fn exhausted_service_order_refuses_new_unit_and_grant_atomically() {
        let mut queue = ServiceOpportunityQueue::default();
        let a = register(&mut queue, "A", 10);
        let b = register(&mut queue, "B", 0);
        queue.last_service_order = u64::MAX - 1;
        assert_eq!(grant(&mut queue, a).ticket, a);
        assert_eq!(queue.last_service_order, u64::MAX);
        let last_ticket = queue.last_ticket;
        assert_invalid(
            queue.register(key("C"), 0),
            "service opportunity service-order identity exhausted",
        );
        assert_eq!(queue.last_ticket, last_ticket);
        assert_eq!(queue.pending_len(), 1);
        // Existing-unit registration needs no new service-order identity.
        let same_unit = register(&mut queue, "B", 0);
        assert_invalid(
            queue.try_grant(b),
            "service opportunity service-order identity exhausted",
        );
        assert_eq!(queue.pending_len(), 2);
        assert_eq!(queue.units.len(), 1);
        assert_eq!(queue.units[0].bypasses, 1);
        assert_eq!(queue.units[0].members[0].ticket, b);
        assert_eq!(queue.units[0].members[1].ticket, same_unit);
        assert!(!queue.busy);
        assert_eq!(queue.last_service_order, u64::MAX);
    }
}
