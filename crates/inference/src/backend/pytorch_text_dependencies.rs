//! Presence prerequisites of the embedded, local Transformers text loader.

use pyo3::prelude::*;

use super::BackendError;

/// Distribution presence requirements for the embedded local text worker.
/// Presence does not certify imports, versions, model compatibility or readiness.
/// NumPy, SoundFile, Torch and Transformers are unconditional worker imports;
/// Accelerate, Safetensors and Tokenizers support the local CPU loader.
pub const TEXT_GENERATION_PACKAGES: [&str; 7] = [
    "numpy",
    "soundfile",
    "torch",
    "transformers",
    "accelerate",
    "safetensors",
    "tokenizers",
];

/// Observe fixed text prerequisites in the worker's linked PyO3 interpreter.
///
/// This blocking metadata query does not initialize the inference worker, import
/// the packages, install dependencies or consult a host executable override.
/// The caller owns blocking isolation and completion.
///
/// # Errors
/// Returns a configuration error if Python metadata cannot be inspected. Only
/// Python's `PackageNotFoundError` is treated as a missing distribution.
pub fn observe_text_package_presence() -> Result<Vec<String>, BackendError> {
    Python::with_gil(|py| observe_distributions(py, &TEXT_GENERATION_PACKAGES))
        .map_err(|_| BackendError::Config("embedded Python text package observation failed".into()))
}

fn observe_distributions(py: Python<'_>, names: &[&str]) -> PyResult<Vec<String>> {
    let metadata = py.import("importlib.metadata")?;
    let distribution = metadata.getattr("distribution")?;
    let not_found = metadata.getattr("PackageNotFoundError")?;
    observe_with_lookup(py, &distribution, &not_found, names)
}

fn observe_with_lookup(
    py: Python<'_>,
    distribution: &Bound<'_, PyAny>,
    not_found: &Bound<'_, PyAny>,
    names: &[&str],
) -> PyResult<Vec<String>> {
    let mut installed = Vec::new();
    for name in names {
        match distribution.call1((*name,)) {
            Ok(_) => installed.push((*name).to_owned()),
            Err(error) if error.is_instance(py, not_found) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_metadata_query_distinguishes_absence_from_observation_failure() {
        Python::with_gil(|py| {
            // Private module dictionary: no package imports, sys.path mutation,
            // process environment changes or shared metadata monkeypatching.
            let module = pyo3::types::PyModule::new(py, "text_metadata_fixture").unwrap();
            py.run(c"class Missing(Exception): pass\ndef lookup(name):\n    if name == 'missing': raise Missing()\n    if name == 'broken': raise RuntimeError('metadata unavailable')\n    return object()\n", Some(&module.dict()), None).unwrap();
            let lookup = module.getattr("lookup").unwrap();
            let missing = module.getattr("Missing").unwrap();
            assert_eq!(
                observe_with_lookup(py, &lookup, &missing, &["present", "missing"]).unwrap(),
                vec!["present"]
            );
            let error =
                observe_with_lookup(py, &lookup, &missing, &["present", "broken"]).unwrap_err();
            assert!(error.is_instance_of::<pyo3::exceptions::PyRuntimeError>(py));
        });
    }

    #[test]
    fn fixed_text_query_uses_linked_python_metadata() {
        let expected =
            Python::with_gil(|py| observe_distributions(py, &TEXT_GENERATION_PACKAGES)).unwrap();
        assert_eq!(observe_text_package_presence().unwrap(), expected);
    }
}
