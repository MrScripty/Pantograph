use super::*;

const MIB: u64 = 1024 * 1024;

fn ram_mib(mib: u64) -> u64 {
    mib * MIB
}

fn vram_mib(mib: u64) -> u64 {
    mib * MIB
}

fn ram_claim_mib(mib: u64) -> RuntimeReservationResourceClaim {
    RuntimeReservationResourceClaim::ram_bytes(ram_mib(mib))
}

fn vram_claim_mib(mib: u64) -> RuntimeReservationResourceClaim {
    RuntimeReservationResourceClaim::vram_bytes(vram_mib(mib))
}

fn reservation_requirements(
    claims: Vec<RuntimeReservationResourceClaim>,
) -> RuntimeReservationRequirements {
    RuntimeReservationRequirements::from_claims(claims)
}

fn ram_budget_mib(total_mib: Option<u64>) -> RuntimeAdmissionResourceBudget {
    RuntimeAdmissionResourceBudget::ram_bytes(total_mib.map(ram_mib))
}

fn vram_budget_mib(total_mib: Option<u64>) -> RuntimeAdmissionResourceBudget {
    RuntimeAdmissionResourceBudget::vram_bytes(total_mib.map(vram_mib))
}

#[path = "lib_tests/admission.rs"]
mod admission;
#[path = "lib_tests/lifecycle.rs"]
mod lifecycle;
#[path = "lib_tests/observations.rs"]
mod observations;
#[path = "lib_tests/reclaim.rs"]
mod reclaim;
#[path = "lib_tests/reservations.rs"]
mod reservations;
#[path = "lib_tests/retention_warmup.rs"]
mod retention_warmup;
