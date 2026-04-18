defmodule Pantograph.Native do
  @moduledoc false

  @on_load :load_nif

  def load_nif do
    path =
      "PANTOGRAPH_RUSTLER_NIF_PATH"
      |> System.fetch_env!()
      |> strip_extension()
      |> String.to_charlist()

    :erlang.load_nif(path, 0)
  end

  def version, do: :erlang.nif_error(:nif_not_loaded)
  def workflow_new(_id, _name), do: :erlang.nif_error(:nif_not_loaded)
  def workflow_from_json(_graph_json), do: :erlang.nif_error(:nif_not_loaded)

  defp strip_extension(path) do
    case Path.extname(path) do
      "" -> path
      _ -> Path.rootname(path)
    end
  end
end
