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

        for runtime_variant in definition.catalog_runtime_variants() {
            catalog.push(ManagedRuntimeCatalogVersion {
                version: tag.to_string(),
                display_label: catalog_display_label(tag, runtime_variant.display_suffix),
                runtime_key: runtime_key_for(definition),
                runtime_variant_id: runtime_variant.runtime_variant_id,
                platform_key: definition.platform_key().to_string(),
                archive_name: asset.name.clone(),
                download_url: asset.browser_download_url.clone(),
            });
        }
    }

    catalog
}

fn fallback_catalog_version(id: ManagedBinaryId) -> Result<ManagedRuntimeCatalogVersion, String> {
    let definition = definition(id);
    let version = definition.default_release_version().to_string();
    let release_asset = definition.release_asset(&version)?;

    let runtime_variant = definition
        .catalog_runtime_variants()
        .into_iter()
        .next()
        .ok_or_else(|| format!("{} has no catalog runtime variants", id.display_name()))?;

    Ok(ManagedRuntimeCatalogVersion {
        version: version.clone(),
        display_label: catalog_display_label(&version, runtime_variant.display_suffix),
        runtime_key: id.key().to_string(),
        runtime_variant_id: runtime_variant.runtime_variant_id,
        platform_key: definition.platform_key().to_string(),
        archive_name: release_asset.archive_name.clone(),
        download_url: definition.download_url(&version, &release_asset),
    })
}

fn catalog_display_label(version: &str, display_suffix: Option<&str>) -> String {
    display_suffix
        .map(|suffix| format!("{version} {suffix}"))
        .unwrap_or_else(|| version.to_string())
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

        assert_eq!(catalog.len(), expected_llama_catalog_variant_count());
        let cpu = catalog
            .iter()
            .find(|entry| entry.runtime_variant_id.as_str() == "llama_cpp.cpu")
            .expect("cpu catalog entry");
        assert_eq!(cpu.version, "b8248");
        assert_eq!(cpu.display_label, "b8248");
        assert_eq!(cpu.archive_name, "llama-b8248-bin-ubuntu-x64.tar.gz");

        #[cfg(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64")
        ))]
        {
            let cuda = catalog
                .iter()
                .find(|entry| entry.runtime_variant_id.as_str() == "llama_cpp.cuda")
                .expect("cuda catalog entry");
            assert_eq!(cuda.version, "b8248");
            assert_eq!(cuda.display_label, "b8248 CUDA");
            assert_eq!(cuda.archive_name, cpu.archive_name);
        }
    }

    fn expected_llama_catalog_variant_count() -> usize {
        #[cfg(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64")
        ))]
        {
            2
        }
        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64")
        )))]
        {
            1
        }
    }
}
