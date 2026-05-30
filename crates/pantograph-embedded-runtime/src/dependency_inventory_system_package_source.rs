use async_trait::async_trait;
use pantograph_dependency_planning::SystemPackageProviderSourceSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SystemPackageProviderSourceError {
    NotImplemented(String),
    #[allow(dead_code)]
    Unavailable(String),
}

impl SystemPackageProviderSourceError {
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::NotImplemented(message) | Self::Unavailable(message) => message,
        }
    }
}

#[async_trait]
pub(crate) trait SystemPackageProviderSource: Send + Sync {
    async fn snapshot(
        &self,
    ) -> Result<SystemPackageProviderSourceSnapshot, SystemPackageProviderSourceError>;
}

pub(crate) struct NotImplementedSystemPackageProviderSource;

#[async_trait]
impl SystemPackageProviderSource for NotImplementedSystemPackageProviderSource {
    async fn snapshot(
        &self,
    ) -> Result<SystemPackageProviderSourceSnapshot, SystemPackageProviderSourceError> {
        Err(SystemPackageProviderSourceError::NotImplemented(
            "System-package inventory source is not implemented for this host composition."
                .to_string(),
        ))
    }
}
