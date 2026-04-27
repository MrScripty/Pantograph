use std::fmt;

const MAX_WORKFLOW_IDENTITY_LEN: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowIdentity(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowIdentityError {
    value: String,
    reason: &'static str,
}

impl WorkflowIdentity {
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowIdentityError> {
        let value = value.into();
        validate_identity_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl WorkflowIdentityError {
    fn new(value: &str, reason: &'static str) -> Self {
        Self {
            value: value.to_string(),
            reason,
        }
    }

    pub fn reason(&self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for WorkflowIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "workflow_id '{}' is invalid: {}",
            self.value, self.reason
        )
    }
}

impl std::error::Error for WorkflowIdentityError {}

impl TryFrom<String> for WorkflowIdentity {
    type Error = WorkflowIdentityError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for WorkflowIdentity {
    type Error = WorkflowIdentityError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_string())
    }
}

fn validate_identity_value(value: &str) -> Result<(), WorkflowIdentityError> {
    if value.is_empty() {
        return Err(WorkflowIdentityError::new(value, "must be non-empty"));
    }
    if value.trim() != value {
        return Err(WorkflowIdentityError::new(
            value,
            "must not have leading or trailing whitespace",
        ));
    }
    if value.len() > MAX_WORKFLOW_IDENTITY_LEN {
        return Err(WorkflowIdentityError::new(
            value,
            "must be at most 96 bytes",
        ));
    }
    if !value.is_ascii() {
        return Err(WorkflowIdentityError::new(value, "must be ASCII"));
    }

    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(WorkflowIdentityError::new(value, "must be non-empty"));
    };
    let last = value.chars().last().unwrap_or(first);
    if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
        return Err(WorkflowIdentityError::new(
            value,
            "must start and end with an ASCII letter or number",
        ));
    }

    if value
        .chars()
        .any(|character| !is_allowed_identity_character(character))
    {
        return Err(WorkflowIdentityError::new(
            value,
            "may contain only ASCII letters, numbers, '.', '-', and '_'",
        ));
    }

    Ok(())
}

fn is_allowed_identity_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
}

#[cfg(test)]
mod tests {
    use super::WorkflowIdentity;

    #[test]
    fn parse_accepts_supported_identity_shapes() {
        for value in [
            "workflow",
            "workflow-1",
            "workflow_1",
            "workflow.1",
            "Workflow.2026_04-27",
        ] {
            assert_eq!(
                WorkflowIdentity::parse(value)
                    .expect("valid workflow identity")
                    .as_str(),
                value
            );
        }
    }

    #[test]
    fn parse_rejects_unsafe_identity_shapes() {
        for value in [
            "",
            " workflow",
            "workflow ",
            "workflow name",
            "workflow/name",
            ".workflow",
            "workflow.",
            "workflow:name",
            "workflow🙂",
        ] {
            assert!(
                WorkflowIdentity::parse(value).is_err(),
                "expected '{value}' to be rejected"
            );
        }
    }
}
