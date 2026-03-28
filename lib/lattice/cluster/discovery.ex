defmodule Lattice.Cluster.Discovery do
  @moduledoc """
  mDNS-based peer discovery.

  Broadcasts presence on multicast UDP and listens for peer announcements.
  Discovered peers are added to the Node peer list.
  """

  use GenServer

  require Logger

  @mdns_addr {224, 0, 0, 251}
  @mdns_port 5353
  @service_name "_lattice._udp.local"
  @announce_interval_ms 10_000

  @type state :: %{
          node_id: String.t(),
          address: String.t(),
          port: pos_integer(),
          metadata: map(),
          socket: :gen_udp.socket() | nil,
          announce_timer: reference() | nil,
          on_discover: (map() -> any()) | nil,
          enabled: boolean()
        }

  # Client API

  @doc """
  Starts the discovery service.

  Options:
  - :node_id - this node's identifier
  - :address - this node's address for peers to connect
  - :port - this node's port
  - :metadata - additional metadata to announce
  - :on_discover - callback when peer discovered (default: adds to Node)
  - :enabled - whether to actually bind UDP (default: true, false for tests)
  """
  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Announces this node's presence to the network.
  """
  @spec announce() :: :ok
  def announce do
    GenServer.cast(__MODULE__, :announce)
  end

  @doc """
  Returns the current state (for debugging).
  """
  @spec get_state() :: state()
  def get_state do
    GenServer.call(__MODULE__, :get_state)
  end

  @doc """
  Manually processes a discovery message (for testing).
  """
  @spec handle_discovery(binary()) :: :ok
  def handle_discovery(message) do
    GenServer.cast(__MODULE__, {:handle_discovery, message})
  end

  @doc """
  Stops the discovery service.
  """
  @spec stop() :: :ok
  def stop do
    GenServer.stop(__MODULE__)
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    node_id = Keyword.get(opts, :node_id, UUID.uuid4())
    address = Keyword.get(opts, :address, "127.0.0.1")
    port = Keyword.get(opts, :port, 4000)
    metadata = Keyword.get(opts, :metadata, %{})
    on_discover = Keyword.get(opts, :on_discover, &default_on_discover/1)
    enabled = Keyword.get(opts, :enabled, true)

    state = %{
      node_id: node_id,
      address: address,
      port: port,
      metadata: metadata,
      socket: nil,
      announce_timer: nil,
      on_discover: on_discover,
      enabled: enabled
    }

    state =
      if enabled do
        case open_socket() do
          {:ok, socket} ->
            state = %{state | socket: socket}
            # Send initial announce
            do_announce(state)
            # Schedule periodic announces
            schedule_announce(state)

          {:error, reason} ->
            Logger.warning("Discovery: failed to open UDP socket: #{inspect(reason)}")
            state
        end
      else
        state
      end

    {:ok, state}
  end

  @impl true
  def handle_cast(:announce, state) do
    do_announce(state)
    {:noreply, state}
  end

  @impl true
  def handle_cast({:handle_discovery, message}, state) do
    process_message(message, state)
    {:noreply, state}
  end

  @impl true
  def handle_call(:get_state, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_info({:udp, _socket, _ip, _port, data}, state) do
    process_message(data, state)
    {:noreply, state}
  end

  @impl true
  def handle_info(:announce, state) do
    do_announce(state)
    state = schedule_announce(state)
    {:noreply, state}
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  @impl true
  def terminate(_reason, state) do
    if state.socket do
      :gen_udp.close(state.socket)
    end

    if state.announce_timer do
      Process.cancel_timer(state.announce_timer)
    end

    :ok
  end

  # Private functions

  defp open_socket do
    opts = [
      :binary,
      active: true,
      reuseaddr: true,
      multicast_ttl: 4,
      multicast_loop: true,
      add_membership: {@mdns_addr, {0, 0, 0, 0}}
    ]

    :gen_udp.open(@mdns_port, opts)
  end

  defp do_announce(%{socket: nil}), do: :ok

  defp do_announce(state) do
    message = encode_announcement(state)

    if state.socket do
      :gen_udp.send(state.socket, @mdns_addr, @mdns_port, message)
    end

    :ok
  end

  defp schedule_announce(state) do
    if state.announce_timer do
      Process.cancel_timer(state.announce_timer)
    end

    ref = Process.send_after(self(), :announce, @announce_interval_ms)
    %{state | announce_timer: ref}
  end

  defp encode_announcement(state) do
    announcement = %{
      service: @service_name,
      node_id: state.node_id,
      address: state.address,
      port: state.port,
      metadata: state.metadata,
      timestamp: System.system_time(:millisecond)
    }

    :erlang.term_to_binary(announcement)
  end

  defp process_message(data, state) do
    try do
      announcement = :erlang.binary_to_term(data, [:safe])
      handle_announcement(announcement, state)
    rescue
      _ -> :ok
    end
  end

  defp handle_announcement(%{service: @service_name} = announcement, state) do
    # Ignore our own announcements
    if announcement.node_id != state.node_id do
      peer = %{
        id: announcement.node_id,
        address: announcement.address,
        port: announcement.port,
        metadata: announcement.metadata
      }

      if state.on_discover do
        state.on_discover.(peer)
      end
    end

    :ok
  end

  defp handle_announcement(_other, _state), do: :ok

  defp default_on_discover(peer) do
    # Add to Node's peer list if Node is running
    if Process.whereis(Lattice.Cluster.Node) do
      Lattice.Cluster.Node.add_peer(peer)
    end
  end
end
