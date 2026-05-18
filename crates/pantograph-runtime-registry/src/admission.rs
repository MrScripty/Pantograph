use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeAdmissionBudget {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<RuntimeAdmissionResourceBudget>,
}

impl RuntimeAdmissionBudget {
    pub fn from_resources(resources: Vec<RuntimeAdmissionResourceBudget>) -> Self {
        Self { resources }
    }

    pub(crate) fn resource_budget(
        &self,
        kind: RuntimeAdmissionResourceKind,
    ) -> Option<&RuntimeAdmissionResourceBudget> {
        self.resources.iter().find(|budget| budget.kind == kind)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAdmissionResourceKind {
    RamBytes,
    VramBytes,
}

impl RuntimeAdmissionResourceKind {
    pub(crate) fn resource_label(self) -> &'static str {
        match self {
            Self::RamBytes => "ram_bytes",
            Self::VramBytes => "vram_bytes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeAdmissionResourceBudget {
    pub kind: RuntimeAdmissionResourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(default)]
    pub safety_margin_bytes: u64,
}

impl RuntimeAdmissionResourceBudget {
    pub fn new(kind: RuntimeAdmissionResourceKind, total_bytes: Option<u64>) -> Self {
        Self {
            kind,
            total_bytes,
            safety_margin_bytes: 0,
        }
    }

    pub fn ram_bytes(total_bytes: Option<u64>) -> Self {
        Self::new(RuntimeAdmissionResourceKind::RamBytes, total_bytes)
    }

    pub fn vram_bytes(total_bytes: Option<u64>) -> Self {
        Self::new(RuntimeAdmissionResourceKind::VramBytes, total_bytes)
    }

    pub fn with_safety_margin_bytes(mut self, safety_margin_bytes: u64) -> Self {
        self.safety_margin_bytes = safety_margin_bytes;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeReservationRequirements {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<RuntimeReservationResourceClaim>,
}

impl RuntimeReservationRequirements {
    pub fn from_claims(claims: Vec<RuntimeReservationResourceClaim>) -> Self {
        Self { claims }
    }

    pub fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeReservationResourceClaim {
    pub kind: RuntimeAdmissionResourceKind,
    pub bytes: u64,
}

impl RuntimeReservationResourceClaim {
    pub fn new(kind: RuntimeAdmissionResourceKind, bytes: u64) -> Self {
        Self { kind, bytes }
    }

    pub fn ram_bytes(bytes: u64) -> Self {
        Self::new(RuntimeAdmissionResourceKind::RamBytes, bytes)
    }

    pub fn vram_bytes(bytes: u64) -> Self {
        Self::new(RuntimeAdmissionResourceKind::VramBytes, bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeAdmissionFailure {
    #[error(
        "insufficient_ram requested={requested_bytes} bytes available={available_bytes} bytes reserved={reserved_bytes} bytes total={total_bytes} bytes safety_margin={safety_margin_bytes} bytes"
    )]
    InsufficientRam {
        requested_bytes: u64,
        available_bytes: u64,
        reserved_bytes: u64,
        total_bytes: u64,
        safety_margin_bytes: u64,
    },
    #[error(
        "insufficient_vram requested={requested_bytes} bytes available={available_bytes} bytes reserved={reserved_bytes} bytes total={total_bytes} bytes safety_margin={safety_margin_bytes} bytes"
    )]
    InsufficientVram {
        requested_bytes: u64,
        available_bytes: u64,
        reserved_bytes: u64,
        total_bytes: u64,
        safety_margin_bytes: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RuntimeReservationClaim {
    pub ram_bytes: Option<u64>,
    pub vram_bytes: Option<u64>,
}
