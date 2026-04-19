use crate::{ElixirExecutionMode, ElixirNodeCategory, ElixirPortDataType};

pub(crate) fn parse_port_data_type_string(type_str: String) -> ElixirPortDataType {
    match type_str.to_lowercase().as_str() {
        "string" => ElixirPortDataType::String,
        "number" => ElixirPortDataType::Number,
        "boolean" => ElixirPortDataType::Boolean,
        "json" => ElixirPortDataType::Json,
        "kv_cache" => ElixirPortDataType::KvCache,
        "image" => ElixirPortDataType::Image,
        "audio" => ElixirPortDataType::Audio,
        "video" => ElixirPortDataType::Video,
        "embedding" => ElixirPortDataType::Embedding,
        "document" => ElixirPortDataType::Document,
        "binary" => ElixirPortDataType::Binary,
        _ => ElixirPortDataType::Any,
    }
}

pub(crate) fn parse_node_category_string(category_str: String) -> ElixirNodeCategory {
    match category_str.to_lowercase().as_str() {
        "input" => ElixirNodeCategory::Input,
        "output" => ElixirNodeCategory::Output,
        "processing" => ElixirNodeCategory::Processing,
        "control" => ElixirNodeCategory::Control,
        "storage" => ElixirNodeCategory::Storage,
        _ => ElixirNodeCategory::Integration,
    }
}

pub(crate) fn parse_execution_mode_string(mode_str: String) -> ElixirExecutionMode {
    match mode_str.to_lowercase().as_str() {
        "manual" => ElixirExecutionMode::Manual,
        "stream" => ElixirExecutionMode::Stream,
        _ => ElixirExecutionMode::Reactive,
    }
}
