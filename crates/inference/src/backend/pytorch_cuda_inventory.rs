//! CUDA identities from the same embedded PyTorch owner used for execution.
//! No Pumas monitoring aggregates, configured selectors or host ordinals enter here.

use std::collections::HashSet;

use pyo3::prelude::*;
use serde::Serialize;

use super::PyTorchBackend;
use crate::InferenceDeviceId;

const MAX_CUDA_DEVICES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PyTorchCudaInventoryUnavailable {
    PythonProbeFailed,
    CudaUnavailable,
    CudaRuntimeNotInitialized,
    UnsupportedHipRuntime,
    InvalidDeviceCount,
    DevicePropertiesFailed,
    ProbeTaskFailed,
}

/// Physical UUID evidence is separate from a backend-visible ordinal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PyTorchCudaPhysicalIdentity {
    /// UUID bytes supplied by the actual CUDA device-properties interface.
    Observed { uuid: String },
    /// Missing/malformed UUIDs cannot be replaced with names or ordinals.
    Unavailable,
    /// Duplicate UUIDs cannot distinguish devices/partitions for comparison.
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PyTorchCudaDeviceFact {
    /// Ordinal in this embedded runtime's visible CUDA namespace only.
    pub device_id: InferenceDeviceId,
    pub physical_identity: PyTorchCudaPhysicalIdentity,
    /// Device-properties total bytes; unavailable differs from observed zero.
    /// This is not free memory, an admission ceiling or an external-use allowance.
    pub total_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PyTorchCudaInventory {
    Observed {
        torch_version: String,
        devices: Vec<PyTorchCudaDeviceFact>,
    },
    Unavailable {
        reason: PyTorchCudaInventoryUnavailable,
    },
}

impl PyTorchBackend {
    /// Explicit one-shot observation in the execution owner's embedded Python.
    /// Nothing runs at construction, discovery of CPU candidates or admission.
    /// PyTorch may cache its CUDA runtime view; this is not host hotplug discovery.
    /// Properties are queried only after CUDA is already initialized, refusing
    /// that API's lazy-initialization path. This flag is not proof of custody.
    pub async fn observe_cuda_inventory() -> PyTorchCudaInventory {
        tokio::task::spawn_blocking(|| {
            Python::with_gil(|py| {
                let Ok(torch) = py.import("torch") else {
                    return unavailable(PyTorchCudaInventoryUnavailable::PythonProbeFailed);
                };
                probe(&torch)
            })
        })
        .await
        .unwrap_or_else(|_| unavailable(PyTorchCudaInventoryUnavailable::ProbeTaskFailed))
    }
}

fn unavailable(reason: PyTorchCudaInventoryUnavailable) -> PyTorchCudaInventory {
    PyTorchCudaInventory::Unavailable { reason }
}
fn probe(torch: &Bound<'_, pyo3::types::PyModule>) -> PyTorchCudaInventory {
    use PyTorchCudaInventoryUnavailable as Reason;
    let result = (|| -> PyResult<_> {
        let version = torch.getattr("__version__")?.extract::<String>()?;
        let hip = torch.getattr("version")?.getattr("hip")?;
        let cuda = torch.getattr("cuda")?;
        let available = cuda.call_method0("is_available")?.extract::<bool>()?;
        let count = cuda.call_method0("device_count")?.extract::<usize>()?;
        Ok((version, !hip.is_none(), cuda, available, count))
    })();
    let Ok((version, hip, cuda, available, count)) = result else {
        return unavailable(Reason::PythonProbeFailed);
    };
    if version.trim().is_empty() || version.len() > 128 || version.chars().any(char::is_control) {
        return unavailable(Reason::PythonProbeFailed);
    }
    if hip {
        return unavailable(Reason::UnsupportedHipRuntime);
    }
    if !available {
        return unavailable(Reason::CudaUnavailable);
    }
    if count == 0 || count > MAX_CUDA_DEVICES {
        return unavailable(Reason::InvalidDeviceCount);
    }
    match cuda
        .call_method0("is_initialized")
        .and_then(|value| value.extract::<bool>())
    {
        Ok(true) => {}
        Ok(false) => return unavailable(Reason::CudaRuntimeNotInitialized),
        Err(_) => return unavailable(Reason::PythonProbeFailed),
    }
    let mut devices = Vec::with_capacity(count);
    let mut identities = HashSet::new();
    let mut duplicates = HashSet::new();
    for ordinal in 0..count {
        let Ok(properties) = cuda.call_method1("get_device_properties", (ordinal,)) else {
            // A partial enumeration cannot become a complete inventory.
            return unavailable(Reason::DevicePropertiesFailed);
        };
        let identity = properties
            .getattr("uuid")
            .and_then(|uuid| uuid.getattr("bytes"))
            .ok()
            .filter(|bytes| bytes.len().ok() == Some(16))
            .and_then(|bytes| bytes.extract::<Vec<u8>>().ok())
            .and_then(|bytes| uuid::Uuid::from_slice(&bytes).ok())
            .filter(|uuid| !uuid.is_nil())
            .map(|uuid| uuid.to_string());
        let physical_identity = match identity {
            Some(uuid) => {
                if !identities.insert(uuid.clone()) {
                    duplicates.insert(uuid.clone());
                }
                PyTorchCudaPhysicalIdentity::Observed { uuid }
            }
            None => PyTorchCudaPhysicalIdentity::Unavailable,
        };
        devices.push(PyTorchCudaDeviceFact {
            device_id: format!("cuda:{ordinal}")
                .parse()
                .expect("bounded CUDA ordinal"),
            physical_identity,
            total_memory_bytes: properties
                .getattr("total_memory")
                .and_then(|value| value.extract::<u64>())
                .ok(),
        });
    }
    for device in &mut devices {
        if let PyTorchCudaPhysicalIdentity::Observed { uuid } = &device.physical_identity {
            if duplicates.contains(uuid) {
                device.physical_identity = PyTorchCudaPhysicalIdentity::Ambiguous;
            }
        }
    }
    PyTorchCudaInventory::Observed {
        torch_version: version,
        devices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn controlled(py: Python<'_>, settings: &str) -> PyTorchCudaInventory {
        let source = format!(
            r#"
import types
__version__ = 'controlled-runtime'
version = types.SimpleNamespace(hip=None)
class Cuda:
    available = True
    initialized = True
    count = 2
    fail_at = None
    uuids = [list(range(16)), list(range(1, 17))]
    memory = [100, 0]
    def is_available(self): return self.available
    def is_initialized(self): return self.initialized
    def device_count(self): return self.count
    def get_device_properties(self, ordinal):
        if ordinal == self.fail_at: raise RuntimeError('controlled property failure')
        return types.SimpleNamespace(uuid=types.SimpleNamespace(bytes=self.uuids[ordinal]), total_memory=self.memory[ordinal])
cuda = Cuda()
{settings}
"#
        );
        let module = pyo3::types::PyModule::from_code(
            py,
            &CString::new(source).unwrap(),
            c"controlled_inventory.py",
            c"controlled_inventory",
        )
        .unwrap();
        probe(&module)
    }
    fn facts(inventory: PyTorchCudaInventory) -> Vec<PyTorchCudaDeviceFact> {
        let PyTorchCudaInventory::Observed { devices, .. } = inventory else {
            panic!("{inventory:?}");
        };
        devices
    }
    #[test]
    fn cuda_inventory_observes_owner_ordinals_uuid_and_known_zero_without_promoting_capacity() {
        Python::with_gil(|py| {
            let devices = facts(controlled(py, ""));
            assert_eq!(devices[0].device_id.as_str(), "cuda:0");
            assert_eq!(devices[1].device_id.as_str(), "cuda:1");
            assert_eq!(devices[0].total_memory_bytes, Some(100));
            assert_eq!(devices[1].total_memory_bytes, Some(0));
            assert_eq!(
                devices[0].physical_identity,
                PyTorchCudaPhysicalIdentity::Observed {
                    uuid: "00010203-0405-0607-0809-0a0b0c0d0e0f".into()
                }
            );
            let reordered = facts(controlled(py, "cuda.uuids.reverse()"));
            assert_eq!(devices[0].physical_identity, reordered[1].physical_identity);
            assert_eq!(devices[1].physical_identity, reordered[0].physical_identity);
        });
    }
    #[test]
    fn cuda_inventory_missing_malformed_or_duplicate_identity_stays_unavailable() {
        Python::with_gil(|py| {
            for settings in [
                "cuda.uuids[0] = []",
                "cuda.uuids[0] = [0]*16",
                "cuda.uuids[0] = [300]*16",
                "cuda.uuids[0] = 'GPU configured name'",
            ] {
                assert_eq!(
                    facts(controlled(py, settings))[0].physical_identity,
                    PyTorchCudaPhysicalIdentity::Unavailable
                );
            }
            let devices = facts(controlled(py, "cuda.uuids[1] = cuda.uuids[0]"));
            assert!(devices
                .iter()
                .all(|row| row.physical_identity == PyTorchCudaPhysicalIdentity::Ambiguous));
            assert_eq!(
                facts(controlled(py, "cuda.memory[0] = None"))[0].total_memory_bytes,
                None
            );
        });
    }
    #[test]
    fn cuda_inventory_rejects_probe_errors_partial_enumeration_hip_and_unbounded_counts() {
        Python::with_gil(|py| {
            for (settings, reason) in [
                (
                    "cuda.available = False",
                    PyTorchCudaInventoryUnavailable::CudaUnavailable,
                ),
                (
                    "cuda.initialized = False; cuda.fail_at = 0",
                    PyTorchCudaInventoryUnavailable::CudaRuntimeNotInitialized,
                ),
                (
                    "version.hip = 'controlled-hip'",
                    PyTorchCudaInventoryUnavailable::UnsupportedHipRuntime,
                ),
                (
                    "cuda.count = 0",
                    PyTorchCudaInventoryUnavailable::InvalidDeviceCount,
                ),
                (
                    "cuda.count = 257",
                    PyTorchCudaInventoryUnavailable::InvalidDeviceCount,
                ),
                (
                    "cuda.fail_at = 1",
                    PyTorchCudaInventoryUnavailable::DevicePropertiesFailed,
                ),
                (
                    "cuda.count = None",
                    PyTorchCudaInventoryUnavailable::PythonProbeFailed,
                ),
                (
                    "__version__ = ''",
                    PyTorchCudaInventoryUnavailable::PythonProbeFailed,
                ),
            ] {
                assert_eq!(controlled(py, settings), unavailable(reason));
            }
        });
    }
}
