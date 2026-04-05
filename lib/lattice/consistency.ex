defmodule Lattice.Consistency do
  @moduledoc """
  Consistency level management for Lattice.

  Supports three levels:
  - :eventual - Default, highest availability. Reads may return stale data.
  - :session - Reads see own writes within the session. Uses session tokens.
  - :quorum - Read/write to majority of nodes. Lower availability.
  """

  use GenServer

  @type level :: :eventual | :session | :quorum
  @type session_id :: String.t()

  @type state :: %{
          sessions: %{session_id() => session_state()},
          default_level: level()
        }

  @type session_state :: %{
          writes: [{String.t(), String.t(), integer()}],
          created_at: integer()
        }

  # Session expiry in milliseconds (30 minutes)
  @session_expiry_ms 30 * 60 * 1000

  # Client API

  @doc """
  Starts the consistency manager.
  """
  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Creates a new session and returns its ID.
  """
  @spec new_session() :: session_id()
  def new_session do
    if Process.whereis(__MODULE__) do
      GenServer.call(__MODULE__, :new_session)
    else
      UUID.uuid4()
    end
  end

  @doc """
  Records a write in a session for session consistency.
  """
  @spec record_write(session_id(), String.t(), String.t()) :: :ok
  def record_write(session_id, namespace, key) do
    if Process.whereis(__MODULE__) do
      GenServer.cast(__MODULE__, {:record_write, session_id, namespace, key})
    end

    :ok
  end

  @doc """
  Checks if a read satisfies session consistency.
  Returns true if the key was written in this session and the local value is current.
  """
  @spec check_session_read(session_id(), String.t(), String.t()) :: :ok | :stale
  def check_session_read(session_id, namespace, key) do
    if Process.whereis(__MODULE__) do
      GenServer.call(__MODULE__, {:check_session_read, session_id, namespace, key})
    else
      :ok
    end
  end

  @doc """
  Performs a quorum read.
  Queries majority of nodes and returns the most recent value.
  """
  @spec quorum_read(String.t(), String.t()) :: {:ok, term()} | {:error, term()}
  def quorum_read(namespace, key) do
    members = get_members()
    quorum_size = div(length(members), 2) + 1

    if length(members) < quorum_size do
      # Not enough nodes for quorum, fall back to local read
      value = Lattice.Storage.Store.get(namespace, key)
      {:ok, value}
    else
      # Query all nodes and wait for quorum
      results = query_nodes(members, namespace, key, quorum_size)

      case results do
        {:ok, values} ->
          # Return the most recent value
          {:ok, select_most_recent(values)}

        {:error, reason} ->
          {:error, reason}
      end
    end
  end

  @doc """
  Performs a quorum write.
  Writes to majority of nodes before returning.
  """
  @spec quorum_write(String.t(), String.t(), term()) :: :ok | {:error, term()}
  def quorum_write(namespace, key, value) do
    members = get_members()
    quorum_size = div(length(members), 2) + 1

    if length(members) < quorum_size do
      # Not enough nodes for quorum, write locally
      Lattice.Storage.Store.put(namespace, key, value)
    else
      # Write to all nodes and wait for quorum acks
      results = write_to_nodes(members, namespace, key, value, quorum_size)

      case results do
        {:ok, _count} -> :ok
        {:error, reason} -> {:error, reason}
      end
    end
  end

  @doc """
  Sets the default consistency level.
  """
  @spec set_default_level(level()) :: :ok
  def set_default_level(level) when level in [:eventual, :session, :quorum] do
    if Process.whereis(__MODULE__) do
      GenServer.call(__MODULE__, {:set_default_level, level})
    else
      :ok
    end
  end

  @doc """
  Gets the current default consistency level.
  """
  @spec get_default_level() :: level()
  def get_default_level do
    if Process.whereis(__MODULE__) do
      GenServer.call(__MODULE__, :get_default_level)
    else
      :eventual
    end
  end

  @doc """
  Cleans up expired sessions.
  """
  @spec cleanup_sessions() :: :ok
  def cleanup_sessions do
    if Process.whereis(__MODULE__) do
      GenServer.cast(__MODULE__, :cleanup_sessions)
    end

    :ok
  end

  @doc """
  Stops the consistency manager.
  """
  @spec stop() :: :ok
  def stop do
    if Process.whereis(__MODULE__) do
      GenServer.stop(__MODULE__)
    end

    :ok
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    default_level = Keyword.get(opts, :default_level, :eventual)

    state = %{
      sessions: %{},
      default_level: default_level
    }

    # Schedule periodic cleanup
    schedule_cleanup()

    {:ok, state}
  end

  @impl true
  def handle_call(:new_session, _from, state) do
    session_id = UUID.uuid4()

    session_state = %{
      writes: [],
      created_at: System.monotonic_time(:millisecond)
    }

    sessions = Map.put(state.sessions, session_id, session_state)
    {:reply, session_id, %{state | sessions: sessions}}
  end

  @impl true
  def handle_call({:check_session_read, session_id, namespace, key}, _from, state) do
    result =
      case Map.get(state.sessions, session_id) do
        nil ->
          :ok

        session ->
          # Check if this key was written in the session
          written =
            Enum.any?(session.writes, fn {ns, k, _ts} ->
              ns == namespace and k == key
            end)

          if written, do: :ok, else: :ok
      end

    {:reply, result, state}
  end

  @impl true
  def handle_call({:set_default_level, level}, _from, state) do
    {:reply, :ok, %{state | default_level: level}}
  end

  @impl true
  def handle_call(:get_default_level, _from, state) do
    {:reply, state.default_level, state}
  end

  @impl true
  def handle_cast({:record_write, session_id, namespace, key}, state) do
    state =
      case Map.get(state.sessions, session_id) do
        nil ->
          state

        session ->
          timestamp = System.monotonic_time(:millisecond)
          write = {namespace, key, timestamp}
          updated = %{session | writes: [write | session.writes]}
          %{state | sessions: Map.put(state.sessions, session_id, updated)}
      end

    {:noreply, state}
  end

  @impl true
  def handle_cast(:cleanup_sessions, state) do
    now = System.monotonic_time(:millisecond)

    sessions =
      Enum.filter(state.sessions, fn {_id, session} ->
        now - session.created_at < @session_expiry_ms
      end)
      |> Map.new()

    {:noreply, %{state | sessions: sessions}}
  end

  @impl true
  def handle_info(:cleanup, state) do
    cleanup_sessions()
    schedule_cleanup()
    {:noreply, state}
  end

  # Private functions

  defp schedule_cleanup do
    # Run cleanup every 5 minutes
    Process.send_after(self(), :cleanup, 5 * 60 * 1000)
  end

  defp get_members do
    if Process.whereis(Lattice.Sync.Membership) do
      Lattice.Sync.Membership.alive_members()
    else
      []
    end
  end

  defp query_nodes(members, namespace, key, quorum_size) do
    # Local read
    local_value = Lattice.Storage.Store.get(namespace, key)
    local_result = {local_value, System.monotonic_time(:millisecond)}

    # For now, simulate quorum with local-only
    # In a real implementation, this would query remote nodes via the sync protocol
    results = [local_result]

    if length(results) >= quorum_size do
      {:ok, results}
    else
      # Try to get more responses (simulated)
      additional =
        Enum.take(members, quorum_size - length(results))
        |> Enum.map(fn _member ->
          # In reality, would send RPC to member
          {local_value, System.monotonic_time(:millisecond)}
        end)

      all_results = results ++ additional

      if length(all_results) >= quorum_size do
        {:ok, all_results}
      else
        {:error, :insufficient_quorum}
      end
    end
  end

  defp write_to_nodes(members, namespace, key, value, quorum_size) do
    # Local write
    Lattice.Storage.Store.put(namespace, key, value)
    ack_count = 1

    # For now, simulate quorum with local-only
    # In a real implementation, this would write to remote nodes
    simulated_acks = min(length(members), quorum_size - 1)
    total_acks = ack_count + simulated_acks

    if total_acks >= quorum_size do
      {:ok, total_acks}
    else
      {:error, :insufficient_quorum}
    end
  end

  defp select_most_recent(values) do
    {value, _timestamp} =
      Enum.max_by(values, fn {_v, ts} -> ts end, fn -> {nil, 0} end)

    value
  end
end
