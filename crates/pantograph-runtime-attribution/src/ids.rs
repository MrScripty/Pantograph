use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AttributionError;

pub(crate) const DEFAULT_BUCKET_NAME: &str = "default";
pub(crate) const MAX_ID_LEN: usize = 128;
pub(crate) const MAX_NAME_LEN: usize = 128;
pub(crate) const MAX_REASON_LEN: usize = 512;

macro_rules! attribution_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn generate() -> Self {
                Self(format!("{}{}", $prefix, Uuid::new_v4()))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = AttributionError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                validate_id(stringify!($name), value).map(Self)
            }
        }

        impl FromStr for $name {
            type Err = AttributionError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_from(value.to_string())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

attribution_id!(ClientId, "client_");
attribution_id!(ClientCredentialId, "cred_");
attribution_id!(ClientSessionId, "session_");
attribution_id!(BucketId, "bucket_");
attribution_id!(WorkflowRunId, "run_");
attribution_id!(WorkflowRunSnapshotId, "runsnap_");
attribution_id!(WorkflowId, "workflow_");
attribution_id!(WorkflowVersionId, "wfver_");
attribution_id!(WorkflowPresentationRevisionId, "wfpres_");
attribution_id!(UsageEventId, "usage_");

pub(crate) fn validate_id(field: &'static str, value: String) -> Result<String, AttributionError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AttributionError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(AttributionError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AttributionError::InvalidField { field });
    }
    Ok(trimmed.to_string())
}

pub(crate) fn validate_bucket_name(value: &str) -> Result<String, AttributionError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AttributionError::MissingField { field: "name" });
    }
    if trimmed.len() > MAX_NAME_LEN {
        return Err(AttributionError::FieldTooLong {
            field: "name",
            max_len: MAX_NAME_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(AttributionError::InvalidField { field: "name" });
    }
    Ok(trimmed.to_string())
}

pub(crate) fn validate_optional_text(
    field: &'static str,
    value: Option<&str>,
    max_len: usize,
) -> Result<(), AttributionError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() > max_len {
        return Err(AttributionError::FieldTooLong { field, max_len });
    }
    if value.chars().any(char::is_control) {
        return Err(AttributionError::InvalidField { field });
    }
    Ok(())
}
