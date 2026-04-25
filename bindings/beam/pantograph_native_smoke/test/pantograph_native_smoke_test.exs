defmodule PantographNativeSmokeTest do
  use ExUnit.Case, async: false

  test "loads the Pantograph Rustler NIF and returns a version" do
    version = Pantograph.Native.version()

    assert is_binary(version)
    assert String.length(version) > 0
  end

  test "workflow_new round-trips through workflow_from_json" do
    graph_json = Pantograph.Native.workflow_new("beam-smoke", "BEAM Smoke")

    assert is_binary(graph_json)
    assert String.contains?(graph_json, "\"id\":\"beam-smoke\"")
    assert String.contains?(graph_json, "\"name\":\"BEAM Smoke\"")
    assert Pantograph.Native.workflow_from_json(graph_json) == graph_json
  end

  test "workflow_validate returns edge-reference errors for unknown nodes" do
    graph_json =
      Pantograph.Native.workflow_new("beam-invalid", "BEAM Invalid")
      |> Pantograph.Native.workflow_add_edge("missing-source", "out", "missing-target", "in")

    errors = Pantograph.Native.workflow_validate(graph_json)

    assert is_list(errors)
    assert Enum.any?(errors, &String.contains?(&1, "unknown node 'missing-source'"))
    assert Enum.any?(errors, &String.contains?(&1, "unknown node 'missing-target'"))
  end

  test "workflow_from_json returns parse errors as BEAM tuples" do
    assert {:error, message} = Pantograph.Native.workflow_from_json("{")
    assert String.contains?(message, "Parse error:")
  end

  test "node registry projects backend-owned graph-authoring discovery" do
    definitions = Pantograph.Native.node_registry_list_definitions()

    assert is_binary(definitions)
    assert String.contains?(definitions, ~s("node_type":"text-input"))
    assert String.contains?(definitions, ~s("io_binding_origin":"client_session"))

    text_input = Pantograph.Native.node_registry_get_definition("text-input")

    assert String.contains?(text_input, ~s("category":"input"))
    assert String.contains?(text_input, ~s("id":"text"))

    grouped = Pantograph.Native.node_registry_definitions_by_category()

    assert String.contains?(grouped, ~s("input"))
    assert String.contains?(grouped, ~s("node_type":"text-input"))

    registry = Pantograph.Native.node_registry_new()
    assert :ok = Pantograph.Native.node_registry_register_builtins(registry)

    queryable = Pantograph.Native.node_registry_queryable_ports(registry)

    assert String.contains?(queryable, ~s("node_type":"puma-lib"))
    assert String.contains?(queryable, ~s("port_id":"model_path"))
  end
end
