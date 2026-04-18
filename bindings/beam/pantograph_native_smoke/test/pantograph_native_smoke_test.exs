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
end
