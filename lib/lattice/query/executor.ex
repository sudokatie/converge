defmodule Lattice.Query.Executor do
  @moduledoc """
  Executes parsed queries against Lattice CRDT data.

  ## Example

      alias Lattice.Query.{Parser, Executor}

      {:ok, query} = Parser.parse("SELECT name FROM users WHERE active = true")
      results = Executor.execute(query, data_source)
  """

  alias Lattice.CRDT.{LWWMap, ORSet}

  @type data_source :: %{String.t() => any()}
  @type result :: {:ok, [map()]} | {:ok, integer()} | {:error, String.t()}

  @doc """
  Execute a parsed query against a data source.

  The data source is a map of collection names to CRDT values.
  """
  def execute(%{select: select, from: from, where: where, limit: limit}, data_source) do
    with {:ok, collection} <- get_collection(data_source, from),
         {:ok, items} <- to_items(collection),
         {:ok, filtered} <- apply_where(items, where),
         {:ok, projected} <- apply_select(filtered, select),
         {:ok, limited} <- apply_limit(projected, limit) do
      {:ok, limited}
    end
  end

  defp get_collection(data_source, name) do
    case Map.get(data_source, name) do
      nil -> {:error, "Collection '#{name}' not found"}
      collection -> {:ok, collection}
    end
  end

  defp to_items(%LWWMap{} = map) do
    items =
      map
      |> LWWMap.keys()
      |> Enum.map(fn key ->
        value = LWWMap.get(map, key)

        case value do
          %{} = m -> Map.put(m, "_key", key)
          _ -> %{"_key" => key, "_value" => value}
        end
      end)

    {:ok, items}
  end

  defp to_items(%ORSet{} = set) do
    items =
      set
      |> ORSet.members()
      |> Enum.map(fn value ->
        case value do
          %{} = m -> m
          _ -> %{"_value" => value}
        end
      end)

    {:ok, items}
  end

  defp to_items(items) when is_list(items) do
    normalized =
      Enum.map(items, fn
        %{} = m -> m
        value -> %{"_value" => value}
      end)

    {:ok, normalized}
  end

  defp to_items(%{} = map) do
    items =
      Enum.map(map, fn {key, value} ->
        case value do
          %{} = m -> Map.put(m, "_key", key)
          _ -> %{"_key" => key, "_value" => value}
        end
      end)

    {:ok, items}
  end

  defp to_items(_), do: {:error, "Unsupported collection type"}

  defp apply_where(items, nil), do: {:ok, items}

  defp apply_where(items, condition) do
    filtered = Enum.filter(items, &evaluate_condition(&1, condition))
    {:ok, filtered}
  end

  defp evaluate_condition(_item, nil), do: true

  defp evaluate_condition(item, {:eq, field, value}) do
    get_field(item, field) == value
  end

  defp evaluate_condition(item, {:neq, field, value}) do
    get_field(item, field) != value
  end

  defp evaluate_condition(item, {:lt, field, value}) do
    get_field(item, field) < value
  end

  defp evaluate_condition(item, {:gt, field, value}) do
    get_field(item, field) > value
  end

  defp evaluate_condition(item, {:lte, field, value}) do
    get_field(item, field) <= value
  end

  defp evaluate_condition(item, {:gte, field, value}) do
    get_field(item, field) >= value
  end

  defp evaluate_condition(item, {:and, left, right}) do
    evaluate_condition(item, left) and evaluate_condition(item, right)
  end

  defp evaluate_condition(item, {:or, left, right}) do
    evaluate_condition(item, left) or evaluate_condition(item, right)
  end

  defp evaluate_condition(item, {:not, expr}) do
    not evaluate_condition(item, expr)
  end

  defp get_field(item, field) when is_binary(field) do
    # Try string key first, then atom
    Map.get(item, field) || Map.get(item, String.to_atom(field))
  end

  defp get_field(_item, _field), do: nil

  defp apply_select(items, [:all]) do
    {:ok, items}
  end

  defp apply_select(items, [{:function, :count, [:all]}]) do
    {:ok, length(items)}
  end

  defp apply_select(items, [{:function, :keys, [:all]}]) do
    keys =
      items
      |> Enum.flat_map(&Map.keys/1)
      |> Enum.uniq()

    {:ok, keys}
  end

  defp apply_select(items, [{:function, :values, [:all]}]) do
    values =
      items
      |> Enum.flat_map(&Map.values/1)
      |> Enum.uniq()

    {:ok, values}
  end

  defp apply_select(items, fields) when is_list(fields) do
    projected =
      Enum.map(items, fn item ->
        fields
        |> Enum.map(fn field ->
          {field, get_field(item, field)}
        end)
        |> Map.new()
      end)

    {:ok, projected}
  end

  defp apply_limit(items, nil), do: {:ok, items}

  defp apply_limit(items, n) when is_integer(n) and is_list(items) do
    {:ok, Enum.take(items, n)}
  end

  defp apply_limit(items, _), do: {:ok, items}
end
