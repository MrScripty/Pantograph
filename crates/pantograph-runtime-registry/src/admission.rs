use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeAdmissionBudget {
    #[serde(default)]
    pub total_ram_mb: Option<u64>,
    #[serde(default)]
    pub total_vram_mb: Option<u64>,
    #[serde(default)]
    pub safety_margin_ram_mb: u64,
    #[serde(default)]
    pub safety_margin_vram_mb: u64,
}

impl RuntimeAdmissionBudget {
    pub fn new(total_ram_mb: Option<u64>, total_vram_mb: Option<u64>) -> Self {
        Self {
            total_ram_mb,
            total_vram_mb,
            safety_margin_ram_mb: 0,
            safety_margin_vram_mb: 0,
        }
    }

    pub fn with_safety_margin_ram_mb(mut self, safety_margin_ram_mb: u64) -> Self {
        self.safety_margin_ram_mb = safety_margin_ram_mb;
        self
    }

    pub fn with_safety_margin_vram_mb(mut self, safety_margin_vram_mb: u64) -> Self {
        self.safety_margin_vram_mb = safety_margin_vram_mb;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeReservationRequirements {
    #[serde(default)]
    pub estimated_peak_vram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_peak_ram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_min_vram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_min_ram_mb: Option<u64>,
}

impl RuntimeReservationRequirements {
    pub fn claimed_ram_mb(&self) -> Option<u64> {
        self.estimated_peak_ram_mb.or(self.estimated_min_ram_mb)
    }

    pub fn claimed_vram_mb(&self) -> Option<u64> {
        self.estimated_peak_vram_mb.or(self.estimated_min_vram_mb)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeAdmissionFailure {
    #[error(
        "insufficient_ram requested={requested_mb}MB available={available_mb}MB reserved={reserved_mb}MB total={total_mb}MB safety_margin={safety_margin_mb}MB"
    )]
    InsufficientRam {
        requested_mb: u64,
        available_mb: u64,
        reserved_mb: u64,
        total_mb: u64,
        safety_margin_mb: u64,
    },
    #[error(
        "insufficient_vram requested={requested_mb}MB available={available_mb}MB reserved={reserved_mb}MB total={total_mb}MB safety_margin={safety_margin_mb}MB"
    )]
    InsufficientVram {
        requested_mb: u64,
        available_mb: u64,
        reserved_mb: u64,
        total_mb: u64,
        safety_margin_mb: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RuntimeReservationClaim {
    pub ram_mb: Option<u64>,
    pub vram_mb: Option<u64>,
}

impl RuntimeReservationClaim {
    pub fn from_requirements(requirements: Option<&RuntimeReservationRequirements>) -> Self {
        let Some(requirements) = requirements else {
            return Self::default();
        };

        Self {
            ram_mb: requirements.claimed_ram_mb(),
            vram_mb: requirements.claimed_vram_mb(),
        }
    }
}
