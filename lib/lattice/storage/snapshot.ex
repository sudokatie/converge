defmodule Lattice.Storage.Snapshot do
  @moduledoc """
  Snapshot management for Lattice.

  Creates periodic snapshots of the database state for faster recovery.
  """
  use GenServer

  @snapshot_prefix "snapshot_"

  defstruct [:data_dir, :interval_ms, :timer_ref]

  # Client API

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Create a snapshot of current state.
  """
  def create do
    GenServer.call(__MODULE__, :create)
  end

  @doc """
  Restore from the latest snapshot.
  """
  def restore do
    GenServer.call(__MODULE__, :restore)
  end

  @doc """
  Restore from a specific snapshot.
  """
  def restore(snapshot_id) do
    GenServer.call(__MODULE__, {:restore, snapshot_id})
  end

  @doc """
  List all available snapshots.
  """
  def list do
    GenServer.call(__MODULE__, :list)
  end

  @doc """
  Delete old snapshots, keeping only the N most recent.
  """
  def cleanup(keep \\ 5) do
    GenServer.call(__MODULE__, {:cleanup, keep})
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    data_dir = Keyword.get(opts, :data_dir, Lattice.Config.data_dir())
    interval_ms = Keyword.get(opts, :interval_ms, Lattice.Config.get(:snapshot_interval_ms))

    File.mkdir_p!(data_dir)

    state = %__MODULE__{
      data_dir: data_dir,
      interval_ms: interval_ms,
      timer_ref: nil
    }

    # Don't start timer in init - let application control this
    {:ok, state}
  end

  @impl true
  def handle_call(:create, _from, state) do
    snapshot_id = create_snapshot(state.data_dir)
    {:reply, {:ok, snapshot_id}, state}
  end

  @impl true
  def handle_call(:restore, _from, state) do
    result =
      case list_snapshots(state.data_dir) do
        [] -> {:error, :no_snapshots}
        snapshots ->
          latest = List.last(snapshots)
          restore_snapshot(state.data_dir, latest)
      end

    {:reply, result, state}
  end

  @impl true
  def handle_call({:restore, snapshot_id}, _from, state) do
    result = restore_snapshot(state.data_dir, snapshot_id)
    {:reply, result, state}
  end

  @impl true
  def handle_call(:list, _from, state) do
    {:reply, list_snapshots(state.data_dir), state}
  end

  @impl true
  def handle_call({:cleanup, keep}, _from, state) do
    snapshots = list_snapshots(state.data_dir)

    if length(snapshots) > keep do
      to_delete = Enum.take(snapshots, length(snapshots) - keep)

      Enum.each(to_delete, fn snapshot_id ->
        path = snapshot_path(state.data_dir, snapshot_id)
        File.rm(path)
      end)
    end

    {:reply, :ok, state}
  end

  # Private

  defp create_snapshot(data_dir) do
    timestamp = DateTime.utc_now() |> DateTime.to_unix(:millisecond)
    snapshot_id = "#{@snapshot_prefix}#{timestamp}"

    # Collect all data from Store if available
    data =
      if Process.whereis(Lattice.Storage.Store) do
        namespaces = Lattice.Storage.Store.list_namespaces()

        Enum.reduce(namespaces, %{}, fn ns, acc ->
          keys = Lattice.Storage.Store.list_keys(ns)
          entries =
            Enum.map(keys, fn key ->
              {key, Lattice.Storage.Store.get(ns, key)}
            end)
            |> Map.new()

          Map.put(acc, ns, entries)
        end)
      else
        %{}
      end

    # Write snapshot
    path = snapshot_path(data_dir, snapshot_id)
    File.write!(path, :erlang.term_to_binary(data))

    snapshot_id
  end

  defp restore_snapshot(data_dir, snapshot_id) do
    path = snapshot_path(data_dir, snapshot_id)

    case File.read(path) do
      {:ok, content} ->
        data = :erlang.binary_to_term(content)

        # Restore to Store if available
        if Process.whereis(Lattice.Storage.Store) do
          Enum.each(data, fn {namespace, entries} ->
            Lattice.Storage.Store.create_namespace(namespace)

            Enum.each(entries, fn {key, value} ->
              Lattice.Storage.Store.put(namespace, key, value)
            end)
          end)
        end

        {:ok, data}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp list_snapshots(data_dir) do
    case File.ls(data_dir) do
      {:ok, files} ->
        files
        |> Enum.filter(&String.starts_with?(&1, @snapshot_prefix))
        |> Enum.sort()

      {:error, _} ->
        []
    end
  end

  defp snapshot_path(data_dir, snapshot_id) do
    Path.join(data_dir, snapshot_id)
  end
end
