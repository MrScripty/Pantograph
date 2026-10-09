//! Separate executable keeps real embedded PyTorch separate from unit-test stubs.
#![cfg(feature = "backend-pytorch")]

use inference::backend::pytorch::{PyTorchCudaInventory, PyTorchCudaInventoryUnavailable};
use inference::InferenceGateway;

#[tokio::test]
#[ignore = "explicit native qualification requires installed real PyTorch"]
async fn actual_embedded_pytorch_inventory_preserves_gateway_and_cpu_candidates() {
    let gateway = InferenceGateway::new();
    let before = gateway.runtime_owned_device_candidates();
    let backend_before = gateway.current_backend_name().await;
    let observation = gateway.observe_pytorch_cuda_inventory().await;
    pyo3::Python::with_gil(|py| {
        use pyo3::prelude::*;
        let torch = py.import("torch").unwrap();
        let version = torch
            .getattr("__version__")
            .unwrap()
            .extract::<String>()
            .unwrap();
        assert!(version.starts_with(|ch: char| ch.is_ascii_digit()));
        println!("native_owner_torch_version={version}");
    });
    println!(
        "native_owned_cuda_inventory={}",
        serde_json::to_string(&observation).unwrap()
    );
    match observation {
        PyTorchCudaInventory::Unavailable { reason } => {
            assert!(matches!(
                reason,
                PyTorchCudaInventoryUnavailable::CudaUnavailable
                    | PyTorchCudaInventoryUnavailable::CudaRuntimeNotInitialized
            ));
        }
        PyTorchCudaInventory::Observed { devices, .. } => assert!(!devices.is_empty()),
    }
    assert_eq!(gateway.runtime_owned_device_candidates(), before);
    assert_eq!(gateway.current_backend_name().await, backend_before);
}
