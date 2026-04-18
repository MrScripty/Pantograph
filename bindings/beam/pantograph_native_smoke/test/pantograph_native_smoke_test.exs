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
end
