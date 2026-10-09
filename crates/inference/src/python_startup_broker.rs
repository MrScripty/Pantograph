//! Registration ordering and keepalive custody, without native execution authority.
//!
//! Supplied metadata and an arbitrary held Arc are declarations/keepalives, not
//! Pumas provenance or observed interpreter/import evidence. Managed start stays
//! closed. This module opens no file, enters no Python and constructs no proof.
use crate::managed_python_binding::{
    ManagedPythonBindingRefusal, ManagedPythonRuntimeStartRequest, ManagedPythonStartObservation,
};
use std::fmt;
#[cfg(feature = "backend-pytorch")]
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

type HeldBytes = Arc<dyn Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonStartupPhase {
    Unclaimed,
    LegacyClaimed,
    ManagedReserved,
    ProcessPinned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonStartupReservationRefusal {
    InvalidDeclaration,
    LegacyClaimed,
    RegistrationBusy,
    GenerationExhausted,
    StatePoisoned,
}

impl fmt::Display for PythonStartupReservationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidDeclaration => "Python startup declaration is invalid",
            Self::LegacyClaimed => "legacy Python startup already claimed this process",
            Self::RegistrationBusy => {
                "Python startup registration or process custody is already held"
            }
            Self::GenerationExhausted => "Python startup registration generation is exhausted",
            Self::StatePoisoned => "Python startup state is poisoned; adoption refused",
        })
    }
}
impl std::error::Error for PythonStartupReservationRefusal {}

struct ManagedHold {
    generation: u64,
    declaration: ManagedPythonRuntimeStartRequest,
    // Retains the exact supplied object, including a future real owner Arc.
    // It does not authenticate that object's issuer or supply native evidence.
    _bytes: HeldBytes,
}
enum State {
    Unclaimed,
    LegacyClaimed,
    ManagedReserved(ManagedHold),
    ProcessPinned(ManagedHold),
}
impl State {
    fn phase(&self) -> PythonStartupPhase {
        match self {
            Self::Unclaimed => PythonStartupPhase::Unclaimed,
            Self::LegacyClaimed => PythonStartupPhase::LegacyClaimed,
            Self::ManagedReserved(_) => PythonStartupPhase::ManagedReserved,
            Self::ProcessPinned(_) => PythonStartupPhase::ProcessPinned,
        }
    }
}
struct Inner {
    next_generation: u64,
    state: State,
}

/// One process-owner registration slot. Fresh instances model controlled owners;
/// actual legacy/managed entry checks share the private process-lifetime singleton.
/// No operation grants native execution, supplies observed provenance or clears a
/// process pin. A model/worker handle cannot retire a process-owned native image.
pub struct PythonStartupBroker(Mutex<Inner>);

impl Default for PythonStartupBroker {
    fn default() -> Self {
        Self(Mutex::new(Inner {
            next_generation: 0,
            state: State::Unclaimed,
        }))
    }
}

impl PythonStartupBroker {
    pub fn phase(&self) -> Result<PythonStartupPhase, PythonStartupReservationRefusal> {
        self.0
            .lock()
            .map(|inner| inner.state.phase())
            .map_err(|_| PythonStartupReservationRefusal::StatePoisoned)
    }

    /// Claim before entering the GIL, releasing the mutex before any native work.
    /// The claim is sticky even on later startup failure; no past native history
    /// is inferred from a successful claim. Repeated ordinary legacy use is allowed.
    pub fn claim_legacy(&self) -> Result<(), PythonStartupReservationRefusal> {
        let mut inner = self
            .0
            .lock()
            .map_err(|_| PythonStartupReservationRefusal::StatePoisoned)?;
        match inner.state {
            State::Unclaimed | State::LegacyClaimed => {
                inner.state = State::LegacyClaimed;
                Ok(())
            }
            State::ManagedReserved(_) | State::ProcessPinned(_) => {
                Err(PythonStartupReservationRefusal::RegistrationBusy)
            }
        }
    }

    /// Reserve declaration/keepalive custody only. An Arc<()> works as a keepalive
    /// but cannot establish owner issuance, loaded-image provenance or readiness.
    /// A future typed owner adapter must provide the genuine retained source.
    pub fn reserve_declared<T: Send + Sync + 'static>(
        self: &Arc<Self>,
        declaration: &ManagedPythonRuntimeStartRequest,
        bytes: Arc<T>,
    ) -> Result<PythonStartupReservation, PythonStartupReservationRefusal> {
        declaration
            .validate()
            .map_err(|_| PythonStartupReservationRefusal::InvalidDeclaration)?;
        let mut inner = self
            .0
            .lock()
            .map_err(|_| PythonStartupReservationRefusal::StatePoisoned)?;
        match inner.state {
            State::LegacyClaimed => return Err(PythonStartupReservationRefusal::LegacyClaimed),
            State::ManagedReserved(_) | State::ProcessPinned(_) => {
                return Err(PythonStartupReservationRefusal::RegistrationBusy);
            }
            State::Unclaimed => {}
        }
        let generation = inner
            .next_generation
            .checked_add(1)
            .ok_or(PythonStartupReservationRefusal::GenerationExhausted)?;
        inner.next_generation = generation;
        inner.state = State::ManagedReserved(ManagedHold {
            generation,
            declaration: declaration.clone(),
            _bytes: bytes,
        });
        Ok(PythonStartupReservation {
            broker: self.clone(),
            generation,
        })
    }

    /// All phases refuse managed execution. State and a Rust worker flag are
    /// negative observations; neither is actual CPython initialization/import proof.
    /// Invalid metadata is rejected before locking or querying even that flag.
    pub fn closed_start_refusal(
        &self,
        declaration: &ManagedPythonRuntimeStartRequest,
        worker_initialized: impl FnOnce() -> bool,
    ) -> ManagedPythonBindingRefusal {
        if declaration.validate().is_err() {
            return ManagedPythonBindingRefusal::InvalidStartRequest;
        }
        if self.phase().is_err() {
            return ManagedPythonBindingRefusal::RegisteredOwnerCustodyUnavailable;
        }
        // Drop the state mutex before invoking even a Rust observation callback.
        // No phase authorizes execution, so this is never a positive race witness.
        declaration.start_refusal(|| ManagedPythonStartObservation {
            legacy_worker_initialized: worker_initialized(),
        })
    }
}

/// Exclusive non-serializable, non-cloneable preflight transaction. Dropping it
/// before potential exposure rolls back only its own generation. After parking,
/// caller loss or failure cannot clear the broker's retained process custody.
pub struct PythonStartupReservation {
    broker: Arc<PythonStartupBroker>,
    generation: u64,
}

impl PythonStartupReservation {
    pub fn declared_selection(
        &self,
    ) -> Result<ManagedPythonRuntimeStartRequest, PythonStartupReservationRefusal> {
        let inner = self
            .broker
            .0
            .lock()
            .map_err(|_| PythonStartupReservationRefusal::StatePoisoned)?;
        match &inner.state {
            State::ManagedReserved(hold) | State::ProcessPinned(hold)
                if hold.generation == self.generation =>
            {
                Ok(hold.declaration.clone())
            }
            _ => Err(PythonStartupReservationRefusal::RegistrationBusy),
        }
    }

    /// Park custody before any future potentially native effect. This is an
    /// irreversible retention decision, not an initialization/import permit.
    /// The production process singleton has no teardown/reset API; actual OS
    /// retirement ends its lifetime. Controlled tests use fresh broker owners.
    pub fn pin_for_process(self) -> Result<(), PythonStartupReservationRefusal> {
        let mut inner = self
            .broker
            .0
            .lock()
            .map_err(|_| PythonStartupReservationRefusal::StatePoisoned)?;
        match &inner.state {
            State::ManagedReserved(hold) if hold.generation == self.generation => {}
            _ => return Err(PythonStartupReservationRefusal::RegistrationBusy),
        }
        let previous = std::mem::replace(&mut inner.state, State::Unclaimed);
        let State::ManagedReserved(hold) = previous else {
            unreachable!("generation checked while holding the same mutex")
        };
        inner.state = State::ProcessPinned(hold);
        Ok(())
    }
}

impl Drop for PythonStartupReservation {
    fn drop(&mut self) {
        let removed = {
            let Ok(mut inner) = self.broker.0.lock() else {
                // Unknown poisoned state keeps any custody until owner retirement.
                return;
            };
            if matches!(&inner.state, State::ManagedReserved(hold) if hold.generation == self.generation)
            {
                Some(std::mem::replace(&mut inner.state, State::Unclaimed))
            } else {
                None
            }
        };
        // Owner destructors can acquire locks or reenter this broker. Never run
        // them while holding its state mutex, and never reset a process pin.
        drop(removed);
    }
}

#[cfg(feature = "backend-pytorch")]
pub(crate) fn process_startup_broker() -> &'static Arc<PythonStartupBroker> {
    static BROKER: OnceLock<Arc<PythonStartupBroker>> = OnceLock::new();
    BROKER.get_or_init(|| Arc::new(PythonStartupBroker::default()))
}

#[cfg(test)]
#[path = "python_startup_broker_tests.rs"]
mod tests;
