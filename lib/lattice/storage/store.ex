defmodule Lattice.Storage.Store do
  @moduledoc """
  ETS/DETS storage backend for Lattice CRDTs.

  Uses ETS for fast in-memory access and DETS for persistence.
  Each namespace gets its own ETS table.
  """
  use GenServer

  @type namespace :: String.t()
  @type key :: any()

  defstruct [:data_dir, :tables, :dets_tables]

  # Client API

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Get a CRDT by namespace and key.
  """
  def get(namespace, key) do
    GenServer.call(__MODULE__, {:get, namespace, key})
  end

  @doc """
  Store a CRDT.
  """
  def put(namespace, key, crdt) do
    GenServer.call(__MODULE__, {:put, namespace, key, crdt})
  end

  @doc """
  Delete a key.
  """
  def delete(namespace, key) do
    GenServer.call(__MODULE__, {:delete, namespace, key})
  end

  @doc """
  List all keys in a namespace.
  """
  def list_keys(namespace) do
    GenServer.call(__MODULE__, {:list_keys, namespace})
  end

  @doc """
  Create a new namespace.
  """
  def create_namespace(namespace) do
    GenServer.call(__MODULE__, {:create_namespace, namespace})
  end

  @doc """
  Delete a namespace and all its data.
  """
  def delete_namespace(namespace) do
    GenServer.call(__MODULE__, {:delete_namespace, namespace})
  end

  @doc """
  List all namespaces.
  """
  def list_namespaces do
    GenServer.call(__MODULE__, :list_namespaces)
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    data_dir = Keyword.get(opts, :data_dir, Lattice.Config.data_dir())
    File.mkdir_p!(data_dir)

    state = %__MODULE__{
      data_dir: data_dir,
      tables: %{},
      dets_tables: %{}
    }

    {:ok, state}
  end

  @impl true
  def handle_call({:get, namespace, key}, _from, state) do
    result =
      case Map.get(state.tables, namespace) do
        nil ->
          nil

        table ->
          case :ets.lookup(table, key) do
            [{^key, crdt}] -> crdt
            [] -> nil
          end
      end

    {:reply, result, state}
  end

  @impl true
  def handle_call({:put, namespace, key, crdt}, _from, state) do
    state = ensure_namespace(state, namespace)
    table = Map.fetch!(state.tables, namespace)
    dets = Map.fetch!(state.dets_tables, namespace)

    :ets.insert(table, {key, crdt})
    :dets.insert(dets, {key, crdt})

    {:reply, :ok, state}
  end

  @impl true
  def handle_call({:delete, namespace, key}, _from, state) do
    case Map.get(state.tables, namespace) do
      nil ->
        {:reply, :ok, state}

      table ->
        dets = Map.fetch!(state.dets_tables, namespace)
        :ets.delete(table, key)
        :dets.delete(dets, key)
        {:reply, :ok, state}
    end
  end

  @impl true
  def handle_call({:list_keys, namespace}, _from, state) do
    keys =
      case Map.get(state.tables, namespace) do
        nil ->
          []

        table ->
          :ets.tab2list(table) |> Enum.map(fn {k, _v} -> k end)
      end

    {:reply, keys, state}
  end

  @impl true
  def handle_call({:create_namespace, namespace}, _from, state) do
    state = ensure_namespace(state, namespace)
    {:reply, :ok, state}
  end

  @impl true
  def handle_call({:delete_namespace, namespace}, _from, state) do
    state =
      case Map.get(state.tables, namespace) do
        nil ->
          state

        table ->
          dets = Map.fetch!(state.dets_tables, namespace)
          :ets.delete(table)
          :dets.close(dets)

          dets_path = dets_path(state.data_dir, namespace)
          File.rm(dets_path)

          %{
            state
            | tables: Map.delete(state.tables, namespace),
              dets_tables: Map.delete(state.dets_tables, namespace)
          }
      end

    {:reply, :ok, state}
  end

  @impl true
  def handle_call(:list_namespaces, _from, state) do
    {:reply, Map.keys(state.tables), state}
  end

  # Private

  defp ensure_namespace(state, namespace) do
    if Map.has_key?(state.tables, namespace) do
      state
    else
      # Create ETS table
      table = :ets.new(:"lattice_#{namespace}", [:set, :public])

      # Open DETS file
      dets_path = dets_path(state.data_dir, namespace)

      {:ok, dets} =
        :dets.open_file(:"lattice_dets_#{namespace}",
          file: String.to_charlist(dets_path),
          type: :set
        )

      # Load existing data from DETS
      :dets.traverse(dets, fn {key, value} ->
        :ets.insert(table, {key, value})
        :continue
      end)

      %{
        state
        | tables: Map.put(state.tables, namespace, table),
          dets_tables: Map.put(state.dets_tables, namespace, dets)
      }
    end
  end

  defp dets_path(data_dir, namespace) do
    Path.join(data_dir, "#{namespace}.dets")
  end
end
