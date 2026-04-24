use pantograph_node_contracts::{
    check_compatibility, CompatibilityCheck, CompatibilityResult, NodeContractError,
    NodeInstanceId, PortKind,
};

use super::types::{PortDataType, PortDefinition};

pub fn validate_connection(source_type: &PortDataType, target_type: &PortDataType) -> bool {
    source_type
        .to_contract_value_type()
        .compatibility_with(target_type.to_contract_value_type())
        .is_compatible()
}

pub fn check_connection_ports(
    source_node_id: &str,
    source_port: &PortDefinition,
    target_node_id: &str,
    target_port: &PortDefinition,
) -> Result<CompatibilityResult, NodeContractError> {
    let source_node_id = source_node_id.parse::<NodeInstanceId>()?;
    let target_node_id = target_node_id.parse::<NodeInstanceId>()?;
    let source_port = source_port.to_contract_port(PortKind::Output)?;
    let target_port = target_port.to_contract_port(PortKind::Input)?;
    let check =
        CompatibilityCheck::new(source_node_id, &source_port, target_node_id, &target_port)?;
    Ok(check_compatibility(check))
}
