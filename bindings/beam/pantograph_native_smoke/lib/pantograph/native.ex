defmodule Pantograph.Native do
  @moduledoc false

  @on_load :load_nif

  @nif_stubs [
    {:version, 0},
    {:parse_port_data_type, 1},
    {:parse_node_category, 1},
    {:parse_execution_mode, 1},
    {:workflow_new, 2},
    {:workflow_from_json, 1},
    {:workflow_add_node, 6},
    {:workflow_remove_node, 2},
    {:workflow_add_edge, 5},
    {:workflow_remove_edge, 2},
    {:workflow_update_node_data, 3},
    {:workflow_validate, 1},
    {:executor_new, 2},
    {:executor_new_with_timeout, 3},
    {:inference_gateway_new, 2},
    {:executor_new_with_inference, 3},
    {:executor_new_with_inference_timeout, 4},
    {:executor_demand, 2},
    {:executor_demand_async, 3},
    {:executor_update_node_data, 3},
    {:executor_mark_modified, 2},
    {:executor_cache_stats, 1},
    {:executor_get_graph_snapshot, 1},
    {:executor_set_input, 4},
    {:executor_get_output, 3},
    {:callback_respond, 2},
    {:callback_error, 2},
    {:orchestration_store_new, 0},
    {:orchestration_store_with_persistence, 1},
    {:orchestration_store_insert, 2},
    {:orchestration_store_get, 2},
    {:orchestration_store_list, 1},
    {:orchestration_store_remove, 2},
    {:node_registry_new, 0},
    {:node_registry_register, 2},
    {:node_registry_list, 1},
    {:node_registry_register_builtins, 1},
    {:extensions_new, 0},
    {:extensions_setup, 2},
    {:node_registry_query_port_options, 5},
    {:execute_orchestration, 4},
    {:execute_orchestration_with_inference, 5},
    {:orchestration_store_insert_data_graph, 3},
    {:pumas_api_discover, 0},
    {:pumas_api_new, 1},
    {:executor_set_pumas_api, 2},
    {:executor_set_kv_cache_store, 2},
    {:pumas_list_models, 1},
    {:pumas_search_models, 4},
    {:pumas_get_model, 2},
    {:pumas_rebuild_index, 1},
    {:pumas_search_hf, 4},
    {:pumas_get_repo_files, 2},
    {:pumas_start_download, 2},
    {:pumas_get_download_progress, 2},
    {:pumas_cancel_download, 2},
    {:pumas_import_model, 2},
    {:pumas_import_batch, 2},
    {:pumas_get_disk_space, 1},
    {:pumas_get_system_resources, 1},
    {:pumas_is_ollama_running, 1}
  ]

  def load_nif do
    path =
      "PANTOGRAPH_RUSTLER_NIF_PATH"
      |> System.fetch_env!()
      |> strip_extension()
      |> String.to_charlist()

    :erlang.load_nif(path, 0)
  end

  for {name, arity} <- @nif_stubs do
    args = Macro.generate_arguments(arity, __MODULE__)

    def unquote(name)(unquote_splicing(args)) do
      :erlang.nif_error(:nif_not_loaded)
    end
  end

  defp strip_extension(path) do
    case Path.extname(path) do
      "" -> path
      _ -> Path.rootname(path)
    end
  end
end
