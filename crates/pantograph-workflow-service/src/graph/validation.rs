use super::types::PortDataType;

pub fn validate_connection(source_type: &PortDataType, target_type: &PortDataType) -> bool {
    source_type.is_compatible_with(target_type)
}
