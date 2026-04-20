use super::contracts::{ManagedBinaryId, ManagedRuntimeCatalogVersion};
use super::definitions::{definition, ManagedBinaryDefinition};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;

const RELEASE_PAGE_SIZE: usize = 12;

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
}

pub(crate) async fn fetch_managed_runtime_catalog(
    id: ManagedBinaryId,
) -> Result<Vec<ManagedRuntimeCatalogVersion>, String> {
    let definition = definition(id);
    let releases = fetch_github_releases(definition).await?;
    let mut catalog = catalog_versions_from_releases(definition, &releases);

    if catalog.is_empty() {
        catalog.push(fallback_catalog_version(id)?);
    }

    Ok(catalog)
}

async fn fetch_github_releases(
    definition: &'static dyn ManagedBinaryDefinition,
) -> Result<Vec<GithubRelease>, String> {
    let (owner, repo) = definition.github_release_repo();
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases?per_page={RELEASE_PAGE_SIZE}"
    );
    let response = reqwest::Client::new()
        .get(url)
        .header(USER_AGENT, "pantograph-managed-runtime-catalog")
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("Failed to refresh runtime catalog: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to refresh runtime catalog: upstream returned {}",
            response.status()
        ));
    }

    response
        .json::<Vec<GithubRelease>>()
        .await
        .map_err(|error| format!("Failed to parse runtime catalog response: {error}"))
}

fn catalog_versions_from_releases(
    definition: &'static dyn ManagedBinaryDefinition,
    releases: &[GithubRelease],
) -> Vec<ManagedRuntimeCatalogVersion> {
    let mut catalog = Vec::new();

    for release in releases {
        if release.draft || release.prerelease {
            continue;
        }

        let tag = release.tag_name.trim();
        if tag.is_empty() {
            continue;
        }

        let Ok(expected_asset) = definition.release_asset(tag) else {
            continue;
        };
        let Some(asset) = release
            .assets
            .iter()
            .find(|candidate| candidate.name == expected_asset.archive_name)
        else {
            continue;
        };

        catalog.push(ManagedRuntimeCatalogVersion {
            version: tag.to_string(),
            display_label: tag.to_string(),
            runtime_key: runtime_key_for(definition),
            platform_key: definition.platform_key().to_string(),
            archive_name: asset.name.clone(),
            download_url: asset.browser_download_url.clone(),
        });
    }

    catalog
}

fn fallback_catalog_version(id: ManagedBinaryId) -> Result<ManagedRuntimeCatalogVersion, String> {
    let definition = definition(id);
    let version = definition.default_release_version().to_string();
    let release_asset = definition.release_asset(&version)?;

    Ok(ManagedRuntimeCatalogVersion {
        version: version.clone(),
        display_label: version.clone(),
        runtime_key: id.key().to_string(),
        platform_key: definition.platform_key().to_string(),
        archive_name: release_asset.archive_name.clone(),
        download_url: definition.download_url(&version, &release_asset),
    })
}

fn runtime_key_for(definition: &'static dyn ManagedBinaryDefinition) -> String {
    definition
        .display_name()
        .to_ascii_lowercase()
        .replace('.', "_")
}

#[cfg(test)]
mod tests {
    use super::{catalog_versions_from_releases, GithubRelease, GithubReleaseAsset};
    use crate::managed_runtime::definitions::definition;
    use crate::managed_runtime::ManagedBinaryId;

    #[test]
    fn catalog_parser_filters_releases_without_matching_platform_asset() {
        let releases = vec![
            GithubRelease {
                tag_name: "b8248".to_string(),
                draft: false,
                prerelease: false,
                assets: vec![GithubReleaseAsset {
                    name: "llama-b8248-bin-ubuntu-x64.tar.gz".to_string(),
                    browser_download_url: "https://example.test/b8248.tar.gz".to_string(),
                }],
            },
            GithubRelease {
                tag_name: "b8247".to_string(),
                draft: false,
                prerelease: false,
                assets: vec![GithubReleaseAsset {
                    name: "not-the-linux-asset.zip".to_string(),
                    browser_download_url: "https://example.test/other.zip".to_string(),
                }],
            },
        ];

        let catalog =
            catalog_versions_from_releases(definition(ManagedBinaryId::LlamaCpp), &releases);

        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].version, "b8248");
        assert_eq!(catalog[0].archive_name, "llama-b8248-bin-ubuntu-x64.tar.gz");
    }
}
