defmodule Lattice.CRDT.LWWMap do
  @moduledoc """
  Last-Writer-Wins Map CRDT.

  A map where each value is an LWW-Register. Supports put, delete, and get
  operations. Deletes are also timestamped to handle concurrent put/delete.

  ## Example

      map = LWWMap.new("node1")
      map = LWWMap.put(map, "name", "Alice")
      map = LWWMap.put(map, "age", 30)
      LWWMap.get(map, "name")  # => "Alice"
      LWWMap.keys(map)  # => ["age", "name"]
  """

  alias Lattice.CRDT.LWWRegister

  @type t :: %__MODULE__{
          node_id: String.t(),
          entries: %{any() => LWWRegister.t()},
          tombstones: %{any() => {integer(), String.t()}}
        }

  defstruct node_id: nil, entries: %{}, tombstones: %{}

  @doc """
  Create a new empty LWW-Map.
  """
  def new(node_id \\ nil) do
    %__MODULE__{
      node_id: node_id || Lattice.Config.node_id(),
      entries: %{},
      tombstones: %{}
    }
  end

  @doc """
  Put a key-value pair.
  """
  def put(
        %__MODULE__{node_id: node_id, entries: entries, tombstones: tombstones} = map,
        key,
        value
      ) do
    timestamp = System.os_time(:nanosecond)
    register = %LWWRegister{value: value, timestamp: timestamp, node_id: node_id}

    # Check if there's a tombstone that's newer
    case Map.get(tombstones, key) do
      {tomb_ts, _} when tomb_ts >= timestamp ->
        # Tombstone is newer, don't add
        map

      _ ->
        # Add/update the entry
        %{map | entries: Map.put(entries, key, register)}
    end
  end

  @doc """
  Delete a key.
  """
  def delete(%__MODULE__{node_id: node_id, entries: entries, tombstones: tombstones} = map, key) do
    timestamp = System.os_time(:nanosecond)

    # Check if entry exists and is newer than our delete
    case Map.get(entries, key) do
      %LWWRegister{timestamp: entry_ts} when entry_ts > timestamp ->
        # Entry is newer, don't delete
        map

      _ ->
        %{
          map
          | entries: Map.delete(entries, key),
            tombstones: Map.put(tombstones, key, {timestamp, node_id})
        }
    end
  end

  @doc """
  Get the value for a key.
  """
  def get(%__MODULE__{entries: entries}, key) do
    case Map.get(entries, key) do
      nil -> nil
      register -> LWWRegister.get(register)
    end
  end

  @doc """
  Get all keys.
  """
  def keys(%__MODULE__{entries: entries}) do
    Map.keys(entries)
  end

  @doc """
  Check if a key exists.
  """
  def has_key?(%__MODULE__{entries: entries}, key) do
    Map.has_key?(entries, key)
  end

  @doc """
  Get the number of entries.
  """
  def size(%__MODULE__{entries: entries}) do
    map_size(entries)
  end

  @doc """
  Merge two LWW-Maps.
  """
  def merge(%__MODULE__{} = a, %__MODULE__{} = b) do
    # Merge tombstones (keep newer)
    merged_tombstones =
      Map.merge(a.tombstones, b.tombstones, fn _key, {ts_a, node_a}, {ts_b, node_b} ->
        if ts_a >= ts_b, do: {ts_a, node_a}, else: {ts_b, node_b}
      end)

    # Merge entries (keep newer, respecting tombstones)
    all_keys =
      MapSet.union(
        MapSet.new(Map.keys(a.entries)),
        MapSet.new(Map.keys(b.entries))
      )

    merged_entries =
      Enum.reduce(all_keys, %{}, fn key, acc ->
        reg_a = Map.get(a.entries, key)
        reg_b = Map.get(b.entries, key)
        tombstone = Map.get(merged_tombstones, key)

        winner = merge_entry(reg_a, reg_b)

        case {winner, tombstone} do
          {nil, _} -> acc
          {reg, nil} -> Map.put(acc, key, reg)
          {reg, {tomb_ts, _}} when reg.timestamp > tomb_ts -> Map.put(acc, key, reg)
          _ -> acc
        end
      end)

    %{a | entries: merged_entries, tombstones: merged_tombstones}
  end

  defp merge_entry(nil, nil), do: nil
  defp merge_entry(nil, b), do: b
  defp merge_entry(a, nil), do: a
  defp merge_entry(a, b), do: LWWRegister.merge(a, b)

  @doc """
  Serialize to binary.
  """
  def to_binary(%__MODULE__{node_id: node_id, entries: entries, tombstones: tombstones}) do
    serialized_entries =
      Enum.map(entries, fn {key, reg} ->
        {key, LWWRegister.to_binary(reg)}
      end)
      |> Map.new()

    :erlang.term_to_binary({node_id, serialized_entries, tombstones})
  end

  @doc """
  Deserialize from binary.
  """
  def from_binary(binary) when is_binary(binary) do
    {node_id, serialized_entries, tombstones} = :erlang.binary_to_term(binary)

    entries =
      Enum.map(serialized_entries, fn {key, reg_bin} ->
        {key, LWWRegister.from_binary(reg_bin)}
      end)
      |> Map.new()

    %__MODULE__{node_id: node_id, entries: entries, tombstones: tombstones}
  end
end
