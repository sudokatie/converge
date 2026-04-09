defmodule Converge do
  @moduledoc """
  Converge - CRDT database with automatic conflict resolution.

  Provides eventually consistent data types that automatically merge
  without coordination. All operations are local-first, with background
  sync to other nodes.

  ## CRDT Types

  - G-Counter: Grow-only counter
  - PN-Counter: Increment/decrement counter
  - LWW-Register: Last-writer-wins register
  - OR-Set: Observed-remove set
  - LWW-Map: Map of LWW registers

  ## Example

      # Counter operations
      Converge.counter_inc("myapp", "visits")
      Converge.counter_value("myapp", "visits")

      # Set operations
      Converge.set_add("myapp", "tags", "elixir")
      Converge.set_members("myapp", "tags")

      # Map operations
      Converge.map_put("myapp", "user:1", "name", "Alice")
      Converge.map_get("myapp", "user:1", "name")
  """

  @type namespace :: String.t()
  @type key :: String.t()

  # Counter Operations

  @doc """
  Increments a counter by 1.
  Uses PN-Counter internally for decrement support.
  """
  @spec counter_inc(namespace(), key()) :: :ok | {:error, term()}
  defdelegate counter_inc(namespace, key), to: Converge.API

  @doc """
  Increments a counter by the given amount.
  """
  @spec counter_inc(namespace(), key(), pos_integer()) :: :ok | {:error, term()}
  defdelegate counter_inc(namespace, key, amount), to: Converge.API

  @doc """
  Decrements a counter by 1.
  """
  @spec counter_dec(namespace(), key()) :: :ok | {:error, term()}
  defdelegate counter_dec(namespace, key), to: Converge.API

  @doc """
  Decrements a counter by the given amount.
  """
  @spec counter_dec(namespace(), key(), pos_integer()) :: :ok | {:error, term()}
  defdelegate counter_dec(namespace, key, amount), to: Converge.API

  @doc """
  Returns the current counter value.
  """
  @spec counter_value(namespace(), key()) :: integer()
  defdelegate counter_value(namespace, key), to: Converge.API

  # Register Operations

  @doc """
  Sets a register value.
  """
  @spec register_set(namespace(), key(), term()) :: :ok | {:error, term()}
  defdelegate register_set(namespace, key, value), to: Converge.API

  @doc """
  Gets a register value.
  """
  @spec register_get(namespace(), key()) :: term() | nil
  defdelegate register_get(namespace, key), to: Converge.API

  # Set Operations

  @doc """
  Adds an element to a set.
  """
  @spec set_add(namespace(), key(), term()) :: :ok | {:error, term()}
  defdelegate set_add(namespace, key, element), to: Converge.API

  @doc """
  Removes an element from a set.
  """
  @spec set_remove(namespace(), key(), term()) :: :ok | {:error, term()}
  defdelegate set_remove(namespace, key, element), to: Converge.API

  @doc """
  Checks if a set contains an element.
  """
  @spec set_contains?(namespace(), key(), term()) :: boolean()
  defdelegate set_contains?(namespace, key, element), to: Converge.API

  @doc """
  Returns all members of a set.
  """
  @spec set_members(namespace(), key()) :: [term()]
  defdelegate set_members(namespace, key), to: Converge.API

  # Map Operations

  @doc """
  Sets a field in a map.
  """
  @spec map_put(namespace(), key(), term(), term()) :: :ok | {:error, term()}
  defdelegate map_put(namespace, key, field, value), to: Converge.API

  @doc """
  Deletes a field from a map.
  """
  @spec map_delete(namespace(), key(), term()) :: :ok | {:error, term()}
  defdelegate map_delete(namespace, key, field), to: Converge.API

  @doc """
  Gets a field value from a map.
  """
  @spec map_get(namespace(), key(), term()) :: term() | nil
  defdelegate map_get(namespace, key, field), to: Converge.API

  @doc """
  Returns all keys in a map.
  """
  @spec map_keys(namespace(), key()) :: [term()]
  defdelegate map_keys(namespace, key), to: Converge.API

  # Namespace Management

  @doc """
  Creates a new namespace.
  """
  @spec create_namespace(String.t(), keyword()) :: :ok | {:error, term()}
  defdelegate create_namespace(name, opts \\ []), to: Converge.API

  @doc """
  Deletes a namespace and all its data.
  """
  @spec delete_namespace(String.t()) :: :ok | {:error, term()}
  defdelegate delete_namespace(name), to: Converge.API

  @doc """
  Lists all namespaces.
  """
  @spec list_namespaces() :: [String.t()] | {:error, term()}
  defdelegate list_namespaces(), to: Converge.API

  # Sync Operations

  @doc """
  Triggers immediate sync for all namespaces.
  """
  @spec sync_now() :: :ok
  defdelegate sync_now(), to: Converge.API

  @doc """
  Triggers immediate sync for a specific namespace.
  """
  @spec sync_now(namespace()) :: :ok
  defdelegate sync_now(namespace), to: Converge.API

  # Cluster Operations

  @doc """
  Returns cluster status information.
  """
  @spec cluster_status() :: map()
  defdelegate cluster_status(), to: Converge.API
end
