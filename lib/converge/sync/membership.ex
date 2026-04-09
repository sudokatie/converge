defmodule Converge.Sync.Membership do
  @moduledoc """
  SWIM-based cluster membership protocol.

  Implements basic SWIM protocol for failure detection:
  - Periodic ping/ping-req to random members
  - Failure detection via timeout
  - Membership gossip on each message

  States: :alive, :suspect, :dead
  """

  use GenServer

  require Logger

  @ping_interval_ms 1_000
  @ping_timeout_ms 500
  @suspect_timeout_ms 2_000

  @type node_id :: String.t()
  @type member_state :: :alive | :suspect | :dead
  @type member :: %{
          id: node_id(),
          address: String.t(),
          port: pos_integer(),
          state: member_state(),
          incarnation: non_neg_integer(),
          last_seen: integer()
        }

  @type state :: %{
          node_id: node_id(),
          members: %{node_id() => member()},
          incarnation: non_neg_integer(),
          ping_timer: reference() | nil,
          pending_pings: %{node_id() => integer()},
          send_fn: (node_id(), term() -> :ok) | nil,
          enabled: boolean()
        }

  # Client API

  @doc """
  Starts the membership manager.

  Options:
  - :node_id - this node's identifier
  - :send_fn - function to send messages to peers (for testing)
  - :enabled - whether to start ping timer (default: true)
  """
  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Joins the cluster via a seed node.
  """
  @spec join(member()) :: :ok
  def join(seed) do
    GenServer.call(__MODULE__, {:join, seed})
  end

  @doc """
  Gracefully leaves the cluster.
  """
  @spec leave() :: :ok
  def leave do
    GenServer.call(__MODULE__, :leave)
  end

  @doc """
  Returns list of all members.
  """
  @spec members() :: [member()]
  def members do
    GenServer.call(__MODULE__, :members)
  end

  @doc """
  Returns list of alive members only.
  """
  @spec alive_members() :: [member()]
  def alive_members do
    GenServer.call(__MODULE__, :alive_members)
  end

  @doc """
  Returns count of alive members.
  """
  @spec member_count() :: non_neg_integer()
  def member_count do
    GenServer.call(__MODULE__, :member_count)
  end

  @doc """
  Handles an incoming membership message.
  """
  @spec handle_message(term()) :: :ok
  def handle_message(message) do
    GenServer.cast(__MODULE__, {:message, message})
  end

  @doc """
  Adds a member directly (for discovery integration).
  """
  @spec add_member(member()) :: :ok
  def add_member(member) do
    GenServer.call(__MODULE__, {:add_member, member})
  end

  @doc """
  Returns current state (for debugging).
  """
  @spec get_state() :: state()
  def get_state do
    GenServer.call(__MODULE__, :get_state)
  end

  @doc """
  Stops the membership manager.
  """
  @spec stop() :: :ok
  def stop do
    GenServer.stop(__MODULE__)
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    node_id = Keyword.get(opts, :node_id, UUID.uuid4())
    send_fn = Keyword.get(opts, :send_fn, nil)
    enabled = Keyword.get(opts, :enabled, true)

    state = %{
      node_id: node_id,
      members: %{},
      incarnation: 0,
      ping_timer: nil,
      pending_pings: %{},
      send_fn: send_fn,
      enabled: enabled
    }

    state =
      if enabled do
        schedule_ping(state)
      else
        state
      end

    {:ok, state}
  end

  @impl true
  def handle_call({:join, seed}, _from, state) do
    member = %{
      id: seed.id,
      address: seed.address,
      port: seed.port,
      state: :alive,
      incarnation: 0,
      last_seen: now()
    }

    members = Map.put(state.members, seed.id, member)

    # Send join message to seed
    if state.send_fn do
      state.send_fn.(seed.id, {:join, state.node_id})
    end

    {:reply, :ok, %{state | members: members}}
  end

  @impl true
  def handle_call(:leave, _from, state) do
    # Broadcast leave to all members
    if state.send_fn do
      for {id, _member} <- state.members do
        state.send_fn.(id, {:leave, state.node_id})
      end
    end

    {:reply, :ok, %{state | members: %{}}}
  end

  @impl true
  def handle_call(:members, _from, state) do
    {:reply, Map.values(state.members), state}
  end

  @impl true
  def handle_call(:alive_members, _from, state) do
    alive =
      state.members
      |> Map.values()
      |> Enum.filter(&(&1.state == :alive))

    {:reply, alive, state}
  end

  @impl true
  def handle_call(:member_count, _from, state) do
    count =
      state.members
      |> Map.values()
      |> Enum.count(&(&1.state == :alive))

    {:reply, count, state}
  end

  @impl true
  def handle_call({:add_member, member}, _from, state) do
    new_member = %{
      id: member.id,
      address: member.address,
      port: member.port,
      state: :alive,
      incarnation: 0,
      last_seen: now()
    }

    members = Map.put(state.members, member.id, new_member)
    {:reply, :ok, %{state | members: members}}
  end

  @impl true
  def handle_call(:get_state, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_cast({:message, {:ping, from_id}}, state) do
    # Respond with ack
    if state.send_fn do
      state.send_fn.(from_id, {:ack, state.node_id})
    end

    state = update_member_seen(state, from_id)
    {:noreply, state}
  end

  @impl true
  def handle_cast({:message, {:ack, from_id}}, state) do
    pending = Map.delete(state.pending_pings, from_id)
    state = update_member_seen(state, from_id)
    {:noreply, %{state | pending_pings: pending}}
  end

  @impl true
  def handle_cast({:message, {:join, from_id}}, state) do
    # New member joined - add them
    new_member = %{
      id: from_id,
      address: "",
      port: 0,
      state: :alive,
      incarnation: 0,
      last_seen: now()
    }

    members = Map.put(state.members, from_id, new_member)
    {:noreply, %{state | members: members}}
  end

  @impl true
  def handle_cast({:message, {:leave, from_id}}, state) do
    members = Map.delete(state.members, from_id)
    {:noreply, %{state | members: members}}
  end

  @impl true
  def handle_cast({:message, _other}, state) do
    {:noreply, state}
  end

  @impl true
  def handle_info(:ping, state) do
    state =
      state
      |> do_ping_round()
      |> check_timeouts()
      |> schedule_ping()

    {:noreply, state}
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  @impl true
  def terminate(_reason, state) do
    if state.ping_timer do
      Process.cancel_timer(state.ping_timer)
    end

    :ok
  end

  # Private functions

  defp schedule_ping(state) do
    if state.ping_timer do
      Process.cancel_timer(state.ping_timer)
    end

    ref = Process.send_after(self(), :ping, @ping_interval_ms)
    %{state | ping_timer: ref}
  end

  defp do_ping_round(state) do
    alive =
      state.members
      |> Map.values()
      |> Enum.filter(&(&1.state == :alive))

    case alive do
      [] ->
        state

      members ->
        # Pick random member to ping
        target = Enum.random(members)

        if state.send_fn do
          state.send_fn.(target.id, {:ping, state.node_id})
        end

        pending = Map.put(state.pending_pings, target.id, now())
        %{state | pending_pings: pending}
    end
  end

  defp check_timeouts(state) do
    current = now()

    # Check pending pings
    {timed_out, still_pending} =
      Enum.split_with(state.pending_pings, fn {_id, sent_at} ->
        current - sent_at > @ping_timeout_ms
      end)

    # Mark timed out as suspect
    members =
      Enum.reduce(timed_out, state.members, fn {id, _}, acc ->
        case Map.get(acc, id) do
          nil -> acc
          member -> Map.put(acc, id, %{member | state: :suspect})
        end
      end)

    # Check suspects for dead timeout
    members =
      Enum.reduce(members, members, fn {id, member}, acc ->
        case member.state do
          :suspect when current - member.last_seen > @suspect_timeout_ms ->
            Map.put(acc, id, %{member | state: :dead})

          _ ->
            acc
        end
      end)

    %{state | members: members, pending_pings: Map.new(still_pending)}
  end

  defp update_member_seen(state, member_id) do
    case Map.get(state.members, member_id) do
      nil ->
        state

      member ->
        updated = %{member | state: :alive, last_seen: now()}
        members = Map.put(state.members, member_id, updated)
        %{state | members: members}
    end
  end

  defp now do
    System.monotonic_time(:millisecond)
  end
end
