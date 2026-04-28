use sysinfo::{Disks, Networks, System};

use crate::scheduler::unix_timestamp_ms;

use super::{
    WorkflowLocalCpuMetrics, WorkflowLocalDiskMetrics, WorkflowLocalGpuMetrics,
    WorkflowLocalMemoryMetrics, WorkflowLocalNetworkInterfaceMetrics,
    WorkflowLocalNetworkNodeStatus, WorkflowLocalNetworkStatusQueryRequest,
    WorkflowLocalNetworkStatusQueryResponse, WorkflowLocalSchedulerLoad,
    WorkflowLocalSystemMetrics, WorkflowNetworkTransportState, WorkflowService,
    WorkflowServiceError,
};

trait LocalSystemMetricsProvider {
    fn capture(
        &mut self,
        request: &WorkflowLocalNetworkStatusQueryRequest,
    ) -> Result<WorkflowLocalSystemMetrics, WorkflowServiceError>;
}

struct SysinfoLocalSystemMetricsProvider;

impl LocalSystemMetricsProvider for SysinfoLocalSystemMetricsProvider {
    fn capture(
        &mut self,
        request: &WorkflowLocalNetworkStatusQueryRequest,
    ) -> Result<WorkflowLocalSystemMetrics, WorkflowServiceError> {
        let mut system = System::new_all();
        system.refresh_memory();
        system.refresh_cpu_all();

        Ok(WorkflowLocalSystemMetrics {
            hostname: System::host_name(),
            os_name: System::name(),
            os_version: System::os_version(),
            kernel_version: System::kernel_version(),
            cpu: local_cpu_metrics(&system),
            memory: WorkflowLocalMemoryMetrics {
                total_bytes: system.total_memory(),
                used_bytes: system.used_memory(),
                available_bytes: system.available_memory(),
            },
            disks: request
                .include_disks
                .then(local_disk_metrics)
                .unwrap_or_default(),
            network_interfaces: request
                .include_network_interfaces
                .then(local_network_interface_metrics)
                .unwrap_or_default(),
            gpu: WorkflowLocalGpuMetrics {
                available: false,
                reason: Some(
                    "GPU metrics are not available from the local system metrics provider"
                        .to_string(),
                ),
            },
        })
    }
}

impl WorkflowService {
    pub fn workflow_local_network_status_query(
        &self,
        request: WorkflowLocalNetworkStatusQueryRequest,
    ) -> Result<WorkflowLocalNetworkStatusQueryResponse, WorkflowServiceError> {
        let mut provider = SysinfoLocalSystemMetricsProvider;
        let system = provider.capture(&request)?;
        let scheduler_load = {
            let store = self.session_store_guard()?;
            WorkflowLocalSchedulerLoad {
                max_sessions: store.max_sessions,
                active_session_count: store.active.len(),
                max_loaded_sessions: store.max_loaded_sessions,
                loaded_session_count: store.loaded_session_count(),
                active_run_count: store.active_run_count(),
                queued_run_count: store.queued_run_count(),
                active_workflow_run_ids: store.active_workflow_run_ids(),
                queued_workflow_run_ids: store.queued_workflow_run_ids(),
            }
        };
        let display_name = system
            .hostname
            .clone()
            .unwrap_or_else(|| "Local Pantograph".to_string());
        let degradation_warnings = system.gpu.reason.clone().into_iter().collect::<Vec<_>>();

        Ok(WorkflowLocalNetworkStatusQueryResponse {
            local_node: WorkflowLocalNetworkNodeStatus {
                node_id: "local".to_string(),
                display_name,
                captured_at_ms: unix_timestamp_ms(),
                transport_state: WorkflowNetworkTransportState::LocalOnly,
                system,
                scheduler_load,
                degradation_warnings,
            },
            peer_nodes: Vec::new(),
        })
    }
}

fn local_cpu_metrics(system: &System) -> WorkflowLocalCpuMetrics {
    let logical_core_count = system.cpus().len();
    let average_usage_percent = if logical_core_count == 0 {
        None
    } else {
        Some(
            system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
                / logical_core_count as f32,
        )
    };

    WorkflowLocalCpuMetrics {
        logical_core_count,
        average_usage_percent,
    }
}

fn local_disk_metrics() -> Vec<WorkflowLocalDiskMetrics> {
    let mut disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| WorkflowLocalDiskMetrics {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
        })
        .collect::<Vec<_>>();
    disks.sort_by(|left, right| {
        left.mount_point
            .cmp(&right.mount_point)
            .then_with(|| left.name.cmp(&right.name))
    });
    disks
}

fn local_network_interface_metrics() -> Vec<WorkflowLocalNetworkInterfaceMetrics> {
    let mut networks = Networks::new_with_refreshed_list()
        .iter()
        .map(|(name, data)| WorkflowLocalNetworkInterfaceMetrics {
            name: name.clone(),
            total_received_bytes: data.total_received(),
            total_transmitted_bytes: data.total_transmitted(),
        })
        .collect::<Vec<_>>();
    networks.sort_by(|left, right| left.name.cmp(&right.name));
    networks
}
