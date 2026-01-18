//! Documentation manager for Svelte 5 docs
//!
//! Handles downloading, caching, and managing local copies of Svelte documentation.

use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::docs_index::SearchIndex;

const SVELTE_DOCS_BASE_URL: &str = "https://raw.githubusercontent.com/sveltejs/svelte/svelte%405.46.3/documentation/docs";
const DOCS_STALENESS_DAYS: i64 = 30;

#[derive(Debug, Error)]
pub enum DocsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Docs not available: {0}")]
    NotAvailable(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocsMetadata {
    pub version: String,
    pub downloaded_at: DateTime<Utc>,
    pub doc_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DocsStatus {
    pub available: bool,
    pub version: Option<String>,
    pub last_updated: Option<String>,
    pub doc_count: usize,
    pub is_stale: bool,
}

/// Manages local Svelte 5 documentation storage and retrieval
#[derive(Clone)]
pub struct DocsManager {
    docs_dir: PathBuf,
}

impl DocsManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            docs_dir: app_data_dir.join("svelte-docs"),
        }
    }

    /// Get the path to the raw docs directory
    fn raw_docs_path(&self) -> PathBuf {
        self.docs_dir.join("docs")
    }

    /// Get the path to the metadata file
    fn metadata_path(&self) -> PathBuf {
        self.docs_dir.join("metadata.json")
    }

    /// Get the path to the search index
    fn index_path(&self) -> PathBuf {
        self.docs_dir.join("index.json")
    }

    /// Check if docs are available. Returns error if not available.
    /// Does NOT auto-download - use the "Download Docs" button in the UI instead.
    /// This prevents disruptive downloads during agent runs that can trigger app rebuilds.
    pub async fn ensure_docs_available(&self) -> Result<(), DocsError> {
        if !self.docs_dir.exists() || !self.index_path().exists() {
            return Err(DocsError::NotAvailable(
                "Svelte docs not downloaded. Use the 'Download Docs' button in the Documentation & RAG panel.".to_string()
            ));
        }

        // Check staleness (just log a warning, don't auto-update)
        if let Ok(metadata) = self.load_metadata() {
            let age = Utc::now().signed_duration_since(metadata.downloaded_at);
            if age.num_days() > DOCS_STALENESS_DAYS {
                log::info!("Svelte docs are stale ({} days old), consider updating via the UI", age.num_days());
            }
        }

        Ok(())
    }

    /// Download Svelte 5 documentation from GitHub
    pub async fn download_docs(&self) -> Result<(), DocsError> {
        log::info!("Downloading Svelte 5 documentation...");

        // Create directories
        let raw_docs = self.raw_docs_path();
        tokio::fs::create_dir_all(&raw_docs).await?;

        // Define the documentation structure to download
        // These are the key sections for Svelte 5
        // Svelte 5.46.3 documentation structure (verified from GitHub)
        let doc_sections = vec![
            ("01-introduction", vec![
                "01-overview.md",
                "02-getting-started.md",
                "03-svelte-files.md",
                "04-svelte-js-files.md",
            ]),
            ("02-runes", vec![
                "01-what-are-runes.md",
                "02-$state.md",
                "03-$derived.md",
                "04-$effect.md",
                "05-$props.md",
                "06-$bindable.md",
                "07-$inspect.md",
                "08-$host.md",
            ]),
            ("03-template-syntax", vec![
                "01-basic-markup.md",
                "02-if.md",
                "03-each.md",
                "04-key.md",
                "05-await.md",
                "06-snippet.md",
                "07-@render.md",
                "08-@html.md",
                "09-@attach.md",
                "10-@const.md",
                "11-@debug.md",
                "12-bind.md",
                "13-use.md",
                "14-transition.md",
                "15-in-and-out.md",
                "16-animate.md",
                "17-style.md",
                "18-class.md",
            ]),
            ("04-styling", vec![
                "01-scoped-styles.md",
                "02-global-styles.md",
                "03-custom-properties.md",
                "04-nested-style-elements.md",
            ]),
            ("05-special-elements", vec![
                "01-svelte-boundary.md",
                "02-svelte-window.md",
                "03-svelte-document.md",
                "04-svelte-body.md",
                "05-svelte-head.md",
                "06-svelte-element.md",
                "07-svelte-options.md",
            ]),
        ];

        let client = reqwest::Client::new();
        let mut doc_count = 0;

        for (section, files) in &doc_sections {
            let section_dir = raw_docs.join(section);
            tokio::fs::create_dir_all(&section_dir).await?;

            for file in files {
                let url = format!("{}/{}/{}", SVELTE_DOCS_BASE_URL, section, file);

                match client.get(&url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            let content = response.text().await?;
                            let file_path = section_dir.join(file);
                            tokio::fs::write(&file_path, &content).await?;
                            doc_count += 1;
                            log::debug!("Downloaded: {}/{}", section, file);
                        } else {
                            log::warn!("Failed to download {}/{}: {}", section, file, response.status());
                        }
                    }
                    Err(e) => {
                        log::warn!("Error downloading {}/{}: {}", section, file, e);
                    }
                }
            }
        }

        // Save metadata
        let metadata = DocsMetadata {
            version: "5.46.3".to_string(),
            downloaded_at: Utc::now(),
            doc_count,
        };
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        tokio::fs::write(self.metadata_path(), metadata_json).await?;

        log::info!("Downloaded {} documentation files", doc_count);
        Ok(())
    }

    /// Build the search index from downloaded docs
    pub async fn build_index(&self) -> Result<SearchIndex, DocsError> {
        log::info!("Building search index...");

        let index = SearchIndex::build_from_docs(&self.raw_docs_path())?;

        // Save the index
        let index_json = serde_json::to_string_pretty(&index)?;
        tokio::fs::write(self.index_path(), index_json).await?;

        log::info!("Built index with {} entries", index.entries.len());
        Ok(index)
    }

    /// Load the search index from disk
    pub fn load_index(&self) -> Result<SearchIndex, DocsError> {
        let index_path = self.index_path();
        if !index_path.exists() {
            return Err(DocsError::NotAvailable("Search index not found".to_string()));
        }

        let content = std::fs::read_to_string(index_path)?;
        let index: SearchIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    /// Load metadata from disk
    fn load_metadata(&self) -> Result<DocsMetadata, DocsError> {
        let metadata_path = self.metadata_path();
        if !metadata_path.exists() {
            return Err(DocsError::NotAvailable("Metadata not found".to_string()));
        }

        let content = std::fs::read_to_string(metadata_path)?;
        let metadata: DocsMetadata = serde_json::from_str(&content)?;
        Ok(metadata)
    }

    /// Get the status of the documentation
    pub fn get_status(&self) -> DocsStatus {
        let metadata = self.load_metadata().ok();
        let index_exists = self.index_path().exists();

        let is_stale = metadata.as_ref().map_or(false, |m| {
            Utc::now().signed_duration_since(m.downloaded_at).num_days() > DOCS_STALENESS_DAYS
        });

        DocsStatus {
            available: index_exists,
            version: metadata.as_ref().map(|m| m.version.clone()),
            last_updated: metadata.as_ref().map(|m| m.downloaded_at.to_rfc3339()),
            doc_count: metadata.map(|m| m.doc_count).unwrap_or(0),
            is_stale,
        }
    }

}
