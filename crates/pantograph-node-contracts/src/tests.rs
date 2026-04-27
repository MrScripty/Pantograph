use super::*;

fn id<T: FromStr<Err = NodeContractError>>(value: &str) -> T {
    value.parse().expect("valid id")
}

fn test_contract() -> NodeTypeContract {
    NodeTypeContract {
        node_type: id("llm-inference"),
        category: NodeCategory::Processing,
        label: "LLM Inference".to_string(),
        description: "Runs an LLM request".to_string(),
        inputs: vec![PortContract::input(
            id("prompt"),
            "Prompt",
            PortValueType::Prompt,
            PortRequirement::Required,
        )],
        outputs: vec![PortContract::output(
            id("response"),
            "Response",
            PortValueType::String,
        )],
        execution_semantics: NodeExecutionSemantics::Reactive,
        capability_requirements: vec![NodeCapabilityRequirement::required("llm")],
        authoring: NodeAuthoringMetadata::default(),
        contract_version: Some("1.0.0".to_string()),
        contract_digest: None,
    }
}

#[test]
fn ids_trim_and_reject_invalid_values() {
    let parsed: NodeTypeId = "  llm-inference  ".parse().expect("valid node type");
    assert_eq!(parsed.as_str(), "llm-inference");

    assert_eq!(
        "".parse::<NodeTypeId>().expect_err("blank id"),
        NodeContractError::MissingIdentifier {
            kind: "node_type_id"
        }
    );
    assert_eq!(
        "bad id".parse::<PortId>().expect_err("space rejected"),
        NodeContractError::InvalidIdentifier { kind: "port_id" }
    );
}

#[test]
fn generated_node_instance_ids_are_backend_owned_and_valid() {
    let generated = NodeInstanceId::generate();
    assert!(generated.as_str().starts_with("node_"));
    assert!(generated.as_str().parse::<NodeInstanceId>().is_ok());
}

#[test]
fn port_value_type_compatibility_matches_backend_rules() {
    assert_eq!(
        PortValueType::Any
            .compatibility_with(PortValueType::KvCache)
            .rule,
        Some(CompatibilityRule::Any)
    );
    assert_eq!(
        PortValueType::Prompt
            .compatibility_with(PortValueType::String)
            .rule,
        Some(CompatibilityRule::PromptString)
    );
    assert_eq!(
        PortValueType::AudioStream
            .compatibility_with(PortValueType::Stream)
            .rule,
        Some(CompatibilityRule::AudioStream)
    );
    assert_eq!(
        PortValueType::Number
            .compatibility_with(PortValueType::String)
            .rule,
        Some(CompatibilityRule::StringCoercion)
    );
    assert!(!PortValueType::KvCache.is_compatible_with(PortValueType::Json));
}

#[test]
fn compatibility_result_carries_structured_rejection() {
    let source = PortContract::output(id("cache"), "Cache", PortValueType::KvCache);
    let target = PortContract::input(
        id("json"),
        "JSON",
        PortValueType::Json,
        PortRequirement::Required,
    );
    let check =
        CompatibilityCheck::new(id("source"), &source, id("target"), &target).expect("valid check");

    let result = check_compatibility(check);

    assert!(!result.is_compatible());
    let rejection = result.rejection.expect("rejection");
    assert_eq!(
        rejection.reason,
        ConnectionRejectionReason::IncompatibleTypes
    );
    assert_eq!(rejection.source_port_id.as_str(), "cache");
    assert_eq!(rejection.target_port_id.as_str(), "json");
}

#[test]
fn compatibility_check_rejects_wrong_port_directions() {
    let input = PortContract::input(
        id("prompt"),
        "Prompt",
        PortValueType::Prompt,
        PortRequirement::Required,
    );
    let output = PortContract::output(id("response"), "Response", PortValueType::String);

    let err = CompatibilityCheck::new(id("source"), &input, id("target"), &output)
        .expect_err("input cannot be source");
    assert_eq!(
        err,
        NodeContractError::WrongPortKind {
            port_id: id("prompt"),
            expected: PortKind::Output,
            actual: PortKind::Input,
        }
    );
}

#[test]
fn node_type_contract_validates_port_directions_and_text() {
    let contract = test_contract();
    contract.validate().expect("valid contract");

    let mut invalid = contract;
    invalid.inputs[0].kind = PortKind::Output;

    assert_eq!(
        invalid.validate().expect_err("wrong direction"),
        NodeContractError::WrongPortKind {
            port_id: id("prompt"),
            expected: PortKind::Input,
            actual: PortKind::Output,
        }
    );
}

#[test]
fn effective_contract_preserves_static_ports_with_diagnostics() {
    let static_contract = test_contract();
    let context = NodeInstanceContext {
        node_instance_id: id("llm-1"),
        node_type: id("llm-inference"),
        graph_revision: Some("rev-1".to_string()),
        configuration: Some(serde_json::json!({"model": "example"})),
    };

    let effective = EffectiveNodeContract::from_static(context, static_contract);

    assert_eq!(effective.inputs.len(), 1);
    assert_eq!(effective.outputs.len(), 1);
    assert_eq!(
        effective.inputs[0].expansion_reasons,
        vec![ContractExpansionReason::StaticContract]
    );
    assert!(effective.diagnostics.warnings.is_empty());
}

#[test]
fn effective_contract_merges_dynamic_ports_without_dropping_static_ports() {
    let static_contract = test_contract();
    let context = NodeInstanceContext {
        node_instance_id: id("llm-1"),
        node_type: id("llm-inference"),
        graph_revision: None,
        configuration: None,
    };
    let dynamic_inputs = vec![
        PortContract::input(
            id("prompt"),
            "Prompt Override",
            PortValueType::String,
            PortRequirement::Required,
        ),
        PortContract::input(
            id("temperature"),
            "Temperature",
            PortValueType::Number,
            PortRequirement::Optional,
        ),
    ];

    let effective = EffectiveNodeContract::from_static_with_dynamic_ports(
        context,
        static_contract,
        Some(dynamic_inputs),
        None,
    )
    .expect("effective contract");

    assert_eq!(effective.inputs.len(), 2);
    assert_eq!(effective.inputs[0].base.label, "Prompt Override");
    assert_eq!(
        effective.inputs[0].expansion_reasons,
        vec![
            ContractExpansionReason::StaticContract,
            ContractExpansionReason::DynamicConfiguration
        ]
    );
    assert_eq!(effective.inputs[1].base.id.as_str(), "temperature");
    assert_eq!(
        effective.inputs[1].expansion_reasons,
        vec![ContractExpansionReason::DynamicConfiguration]
    );
    assert_eq!(
        effective.diagnostics.expansion_reasons,
        vec![ContractExpansionReason::DynamicConfiguration]
    );
    assert_eq!(effective.outputs.len(), 1);
}

#[test]
fn contracts_round_trip_as_snake_case_json() {
    let contract = test_contract();
    let value = serde_json::to_value(&contract).expect("serialize");

    assert_eq!(value["node_type"], "llm-inference");
    assert_eq!(value["execution_semantics"], "reactive");
    assert_eq!(value["inputs"][0]["value_type"], "prompt");

    let parsed: NodeTypeContract = serde_json::from_value(value).expect("deserialize");
    assert_eq!(parsed.node_type.as_str(), "llm-inference");
}

fn composed_contract() -> ComposedNodeContract {
    ComposedNodeContract {
        external_contract: NodeTypeContract {
            node_type: id("tool-loop"),
            category: NodeCategory::Control,
            label: "Tool Loop".to_string(),
            description: "Runs a tool loop through primitive nodes".to_string(),
            inputs: vec![PortContract::input(
                id("prompt"),
                "Prompt",
                PortValueType::Prompt,
                PortRequirement::Required,
            )],
            outputs: vec![PortContract::output(
                id("response"),
                "Response",
                PortValueType::String,
            )],
            execution_semantics: NodeExecutionSemantics::Stream,
            capability_requirements: vec![NodeCapabilityRequirement::required("llm")],
            authoring: NodeAuthoringMetadata::default(),
            contract_version: Some("1.0.0".to_string()),
            contract_digest: Some("digest-tool-loop-v1".to_string()),
        },
        internal_graph: ComposedInternalGraph {
            graph_id: "tool-loop-internal-v1".to_string(),
            nodes: vec![
                ComposedInternalNode {
                    node_id: id("llm"),
                    node_type: id("llm-inference"),
                    label: "LLM".to_string(),
                    contract_version: Some("1.0.0".to_string()),
                    contract_digest: None,
                },
                ComposedInternalNode {
                    node_id: id("tool-executor"),
                    node_type: id("tool-executor"),
                    label: "Tool Executor".to_string(),
                    contract_version: Some("1.0.0".to_string()),
                    contract_digest: None,
                },
            ],
            edges: vec![ComposedInternalEdge {
                source_node_id: id("llm"),
                source_port_id: id("tool_calls"),
                target_node_id: id("tool-executor"),
                target_port_id: id("tool_calls"),
            }],
        },
        port_mappings: ComposedPortMappings {
            inputs: vec![ComposedPortMapping {
                external_port_id: id("prompt"),
                internal_node_id: id("llm"),
                internal_port_id: id("prompt"),
            }],
            outputs: vec![ComposedPortMapping {
                external_port_id: id("response"),
                internal_node_id: id("llm"),
                internal_port_id: id("response"),
            }],
        },
        trace_policy: ComposedTracePolicy::PreservePrimitiveFacts,
        upgrade_metadata: None,
    }
}

#[test]
fn composed_node_contract_validates_external_mapping_to_internal_graph() {
    let contract = composed_contract();

    contract.validate().expect("valid composed contract");

    let value = serde_json::to_value(&contract).expect("serialize");
    assert_eq!(value["trace_policy"], "preserve_primitive_facts");
    assert_eq!(value["external_contract"]["node_type"], "tool-loop");
    assert_eq!(
        value["port_mappings"]["inputs"][0]["external_port_id"],
        "prompt"
    );
}

#[test]
fn composed_node_contract_rejects_missing_external_port_mapping() {
    let mut contract = composed_contract();
    contract.port_mappings.outputs.clear();

    assert_eq!(
        contract.validate().expect_err("missing output mapping"),
        NodeContractError::MissingCompositionPortMapping {
            port_id: id("response"),
            kind: PortKind::Output,
        }
    );
}

#[test]
fn composed_node_contract_rejects_unknown_internal_mapping_node() {
    let mut contract = composed_contract();
    contract.port_mappings.inputs[0].internal_node_id = id("missing-node");

    assert_eq!(
        contract.validate().expect_err("unknown internal node"),
        NodeContractError::UnknownCompositionInternalNode {
            node_id: id("missing-node"),
        }
    );
}

#[test]
fn contract_upgrade_records_validate_outcomes_and_diagnostics() {
    let valid = ContractUpgradeRecord {
        node_type: id("system-prompt"),
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::PreservePrimitiveLineage,
        changes: vec![ContractUpgradeChange::NodeTypeChanged {
            node_id: id("node-a"),
            from: id("system-prompt"),
            to: id("text-input"),
        }],
        diagnostics: Vec::new(),
    };

    valid.validate().expect("valid upgrade");

    let missing_change = ContractUpgradeRecord {
        changes: Vec::new(),
        ..valid.clone()
    };
    assert_eq!(
        missing_change.validate().expect_err("missing change"),
        NodeContractError::MissingContractUpgradeChange
    );

    let missing_diagnostic = ContractUpgradeRecord {
        outcome: ContractUpgradeOutcome::TypedRejection,
        diagnostics_lineage: DiagnosticsLineagePolicy::RejectToAvoidSilentChange,
        diagnostics: Vec::new(),
        ..valid
    };
    assert_eq!(
        missing_diagnostic
            .validate()
            .expect_err("typed rejection needs diagnostic"),
        NodeContractError::MissingContractUpgradeDiagnostic
    );
}
