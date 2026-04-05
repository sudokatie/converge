defmodule Lattice.API do
  @moduledoc """
  Core API implementation for Lattice CRDT operations.

  This module contains the implementation of all public API functions.
  The `Lattice` module delegates to this module for the public interface.
  """

  alias Lattice.CRDT.{PNCounter, LWWRegister, ORSet, LWWMap}
  alias Lattice.Storage.Store
  alias Lattice.Sync.AntiEntropy
  alias Lattice.Cluster.Node

  @type namespace :: String.t()
  @type key :: String.t()

  # Counter Operations

  @doc """
  Increments a counter by 1.
  """
  @spec counter_inc(namespace(), key()) :: :ok | {:error, term()}
  def counter_inc(namespace, key) do
    counter_inc(namespace, key, 1)
  end

  @doc """
  Increments a counter by the given amount.
  """
  @spec counter_inc(namespace(), key(), pos_integer()) :: :ok | {:error, term()}
  def counter_inc(namespace, key, amount) when is_integer(amount) and amount > 0 do
    with_counter(namespace, key, fn counter ->
      PNCounter.inc(counter, amount)
    end)
  end

  @doc """
  Decrements a counter by 1.
  """
  @spec counter_dec(namespace(), key()) :: :ok | {:error, term()}
  def counter_dec(namespace, key) do
    counter_dec(namespace, key, 1)
  end

  @doc """
  Decrements a counter by the given amount.
  """
  @spec counter_dec(namespace(), key(), pos_integer()) :: :ok | {:error, term()}
  def counter_dec(namespace, key, amount) when is_integer(amount) and amount > 0 do
    with_counter(namespace, key, fn counter ->
      PNCounter.dec(counter, amount)
    end)
  end

  @doc """
  Returns the current counter value.
  """
  @spec counter_value(namespace(), key()) :: integer()
  def counter_value(namespace, key) do
    case get_crdt(namespace, key) do
      nil -> 0
      %PNCounter{} = counter -> PNCounter.value(counter)
      _ -> 0
    end
  end

  # Register Operations

  @doc """
  Sets a register value.
  """
  @spec register_set(namespace(), key(), term()) :: :ok | {:error, term()}
  def register_set(namespace, key, value) do
    node_id = get_node_id()
    register = LWWRegister.new(node_id) |> LWWRegister.set(value)
    put_crdt(namespace, key, register)
  end

  @doc """
  Gets a register value.
  """
  @spec register_get(namespace(), key()) :: term() | nil
  def register_get(namespace, key) do
    case get_crdt(namespace, key) do
      nil -> nil
      %LWWRegister{} = register -> LWWRegister.get(register)
      _ -> nil
    end
  end

  # Set Operations

  @doc """
  Adds an element to a set.
  """
  @spec set_add(namespace(), key(), term()) :: :ok | {:error, term()}
  def set_add(namespace, key, element) do
    with_set(namespace, key, fn set ->
      ORSet.add(set, element)
    end)
  end

  @doc """
  Removes an element from a set.
  """
  @spec set_remove(namespace(), key(), term()) :: :ok | {:error, term()}
  def set_remove(namespace, key, element) do
    with_set(namespace, key, fn set ->
      ORSet.remove(set, element)
    end)
  end

  @doc """
  Checks if a set contains an element.
  """
  @spec set_contains?(namespace(), key(), term()) :: boolean()
  def set_contains?(namespace, key, element) do
    case get_crdt(namespace, key) do
      nil -> false
      %ORSet{} = set -> ORSet.contains?(set, element)
      _ -> false
    end
  end

  @doc """
  Returns all members of a set.
  """
  @spec set_members(namespace(), key()) :: [term()]
  def set_members(namespace, key) do
    case get_crdt(namespace, key) do
      nil -> []
      %ORSet{} = set -> ORSet.members(set)
      _ -> []
    end
  end

  # Map Operations

  @doc """
  Sets a field in a map.
  """
  @spec map_put(namespace(), key(), term(), term()) :: :ok | {:error, term()}
  def map_put(namespace, key, field, value) do
    with_map(namespace, key, fn map ->
      LWWMap.put(map, field, value)
    end)
  end

  @doc """
  Deletes a field from a map.
  """
  @spec map_delete(namespace(), key(), term()) :: :ok | {:error, term()}
  def map_delete(namespace, key, field) do
    with_map(namespace, key, fn map ->
      LWWMap.delete(map, field)
    end)
  end

  @doc """
  Gets a field value from a map.
  """
  @spec map_get(namespace(), key(), term()) :: term() | nil
  def map_get(namespace, key, field) do
    case get_crdt(namespace, key) do
      nil -> nil
      %LWWMap{} = map -> LWWMap.get(map, field)
      _ -> nil
    end
  end

  @doc """
  Returns all keys in a map.
  """
  @spec map_keys(namespace(), key()) :: [term()]
  def map_keys(namespace, key) do
    case get_crdt(namespace, key) do
      nil -> []
      %LWWMap{} = map -> LWWMap.keys(map)
      _ -> []
    end
  end

  # Namespace Management

  @doc """
  Creates a new namespace.
  """
  @spec create_namespace(String.t(), keyword()) :: :ok | {:error, term()}
  def create_namespace(name, _opts \\ []) do
    if store_running?() do
      Store.create_namespace(name)
    else
      {:error, :store_not_running}
    end
  end

  @doc """
  Deletes a namespace and all its data.
  """
  @spec delete_namespace(String.t()) :: :ok | {:error, term()}
  def delete_namespace(name) do
    if store_running?() do
      Store.delete_namespace(name)
    else
      {:error, :store_not_running}
    end
  end

  @doc """
  Lists all namespaces.
  """
  @spec list_namespaces() :: [String.t()] | {:error, term()}
  def list_namespaces do
    if store_running?() do
      Store.list_namespaces()
    else
      {:error, :store_not_running}
    end
  end

  # Sync Operations

  @doc """
  Triggers immediate sync for all namespaces.
  """
  @spec sync_now() :: :ok
  def sync_now do
    if Process.whereis(AntiEntropy) do
      AntiEntropy.sync_now()
    else
      :ok
    end
  end

  @doc """
  Triggers immediate sync for a specific namespace.
  """
  @spec sync_now(namespace()) :: :ok
  def sync_now(namespace) do
    if Process.whereis(AntiEntropy) do
      AntiEntropy.sync_now(namespace)
    else
      :ok
    end
  end

  # Cluster Operations

  @doc """
  Returns cluster status information.
  """
  @spec cluster_status() :: map()
  def cluster_status do
    node_info =
      if Process.whereis(Node) do
        Node.info()
      else
        %{id: "unknown", metadata: %{}}
      end

    members =
      if Process.whereis(Lattice.Sync.Membership) do
        Lattice.Sync.Membership.alive_members()
      else
        []
      end

    namespaces =
      if store_running?() do
        Store.list_namespaces()
      else
        []
      end

    %{
      node: node_info,
      members: members,
      member_count: length(members),
      namespaces: namespaces,
      namespace_count: length(namespaces)
    }
  end

  # Private Helpers

  defp get_node_id do
    if Process.whereis(Node) do
      Node.node_id()
    else
      "default"
    end
  end

  defp store_running? do
    Process.whereis(Store) != nil
  end

  defp get_crdt(namespace, key) do
    if store_running?() do
      Store.get(namespace, key)
    else
      nil
    end
  end

  defp put_crdt(namespace, key, crdt) do
    if store_running?() do
      Store.put(namespace, key, crdt)
      update_merkle(namespace, key, crdt)
      :ok
    else
      {:error, :store_not_running}
    end
  end

  defp update_merkle(namespace, key, crdt) do
    if Process.whereis(AntiEntropy) do
      hash = :erlang.term_to_binary(crdt)
      AntiEntropy.update_merkle(namespace, key, hash)
    end
  end

  defp with_counter(namespace, key, fun) do
    node_id = get_node_id()

    counter =
      case get_crdt(namespace, key) do
        nil -> PNCounter.new(node_id)
        %PNCounter{} = c -> c
        _ -> PNCounter.new(node_id)
      end

    updated = fun.(counter)
    put_crdt(namespace, key, updated)
  end

  defp with_set(namespace, key, fun) do
    set =
      case get_crdt(namespace, key) do
        nil -> ORSet.new()
        %ORSet{} = s -> s
        _ -> ORSet.new()
      end

    updated = fun.(set)
    put_crdt(namespace, key, updated)
  end

  defp with_map(namespace, key, fun) do
    map =
      case get_crdt(namespace, key) do
        nil -> LWWMap.new()
        %LWWMap{} = m -> m
        _ -> LWWMap.new()
      end

    updated = fun.(map)
    put_crdt(namespace, key, updated)
  end
end
