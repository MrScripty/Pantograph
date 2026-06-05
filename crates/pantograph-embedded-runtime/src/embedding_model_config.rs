use std::path::PathBuf;

pub fn resolve_configured_embedding_model_path(model_path: &str) -> Result<PathBuf, String> {
    let trimmed = model_path.trim();
    if trimmed.is_empty() {
        return Err("Configured embedding model path is empty".to_string());
    }

    let candidate = PathBuf::from(trimmed);
    if !candidate.exists() {
        return Err(format!(
            "Configured embedding model file does not exist: {}",
            candidate.display()
        ));
    }
    if !candidate.is_file() {
        return Err(format!(
            "Configured embedding model path must be a GGUF file, not a directory: {}",
            candidate.display()
        ));
    }
    if !candidate
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
    {
        return Err(format!(
            "Configured embedding model path must point to a .gguf file: {}",
            candidate.display()
        ));
    }

    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::resolve_configured_embedding_model_path;

    #[test]
    fn resolves_explicit_existing_gguf_file() {
        let temp_dir = std::env::temp_dir().join(format!(
            "pantograph-embedding-model-config-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&temp_dir).expect("temp directory should be created");
        let model_path = temp_dir.join("embed.gguf");
        std::fs::write(&model_path, b"gguf").expect("embedding model file should be written");

        let resolved = resolve_configured_embedding_model_path(
            model_path
                .to_str()
                .expect("temporary embedding path should be utf-8"),
        )
        .expect("configured embedding model path should resolve");

        assert_eq!(resolved, model_path);

        std::fs::remove_file(&model_path)
            .expect("temporary embedding model file should be removed");
        std::fs::remove_dir(&temp_dir).expect("temporary test directory should be removed");
    }

    #[test]
    fn rejects_missing_or_non_gguf_paths_without_discovery() {
        let missing = std::env::temp_dir().join(format!(
            "pantograph-missing-embedding-{}.gguf",
            std::process::id()
        ));
        let error = resolve_configured_embedding_model_path(
            missing
                .to_str()
                .expect("temporary missing path should be utf-8"),
        )
        .expect_err("missing path must fail closed");
        assert!(error.contains("does not exist"));

        let temp_dir = std::env::temp_dir().join(format!(
            "pantograph-embedding-model-config-invalid-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&temp_dir).expect("temp directory should be created");
        let model_path = temp_dir.join("embed.bin");
        std::fs::write(&model_path, b"not gguf").expect("embedding model file should be written");

        let error = resolve_configured_embedding_model_path(
            model_path
                .to_str()
                .expect("temporary embedding path should be utf-8"),
        )
        .expect_err("non-gguf path must fail closed");
        assert!(error.contains(".gguf"));

        let error = resolve_configured_embedding_model_path(
            temp_dir
                .to_str()
                .expect("temporary directory path should be utf-8"),
        )
        .expect_err("directory path must fail closed");
        assert!(error.contains("not a directory"));

        std::fs::remove_file(&model_path).expect("temporary embedding file should be removed");
        std::fs::remove_dir(&temp_dir).expect("temporary test directory should be removed");
    }
}
