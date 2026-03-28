defmodule Lattice.Cluster.Node do
  @moduledoc """
  Node identity and peer management.

  Maintains this node's identity (ID, metadata) and tracks known peers.
  Node ID persists across restarts via the data directory.
  """

  use GenServer

  @node_id_file "node_id"

  @type node_id :: String.t()
  @type peer :: %{
          id: node_id(),
          address: String.t(),
          port: pos_integer(),
          metadata: map()
        }

  @type state :: %{
          node_id: node_id(),
          data_dir: String.t(),
          metadata: map(),
          peers: %{node_id() => peer()}
        }

  # Client API

  @doc """
  Starts the node manager.

  Options:
  - :node_id - explicit node ID (default: loaded from disk or generated)
  - :data_dir - directory for persistent state
  - :metadata - node metadata (address, port, etc.)
  """
  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Returns this node's ID.
  """
  @spec node_id() :: node_id()
  def node_id do
    GenServer.call(__MODULE__, :node_id)
  end

  @doc """
  Returns this node's metadata.
  """
  @spec metadata() :: map()
  def metadata do
    GenServer.call(__MODULE__, :metadata)
  end

  @doc """
  Updates node metadata.
  """
  @spec set_metadata(map()) :: :ok
  def set_metadata(metadata) do
    GenServer.call(__MODULE__, {:set_metadata, metadata})
  end

  @doc """
  Returns full node info (id + metadata).
  """
  @spec info() :: %{id: node_id(), metadata: map()}
  def info do
    GenServer.call(__MODULE__, :info)
  end

  @doc """
  Adds a peer to the known peer list.
  """
  @spec add_peer(peer()) :: :ok
  def add_peer(peer) do
    GenServer.call(__MODULE__, {:add_peer, peer})
  end

  @doc """
  Removes a peer by ID.
  """
  @spec remove_peer(node_id()) :: :ok
  def remove_peer(peer_id) do
    GenServer.call(__MODULE__, {:remove_peer, peer_id})
  end

  @doc """
  Returns list of all known peers.
  """
  @spec peers() :: [peer()]
  def peers do
    GenServer.call(__MODULE__, :peers)
  end

  @doc """
  Returns a specific peer by ID.
  """
  @spec get_peer(node_id()) :: peer() | nil
  def get_peer(peer_id) do
    GenServer.call(__MODULE__, {:get_peer, peer_id})
  end

  @doc """
  Returns list of peer IDs.
  """
  @spec peer_ids() :: [node_id()]
  def peer_ids do
    GenServer.call(__MODULE__, :peer_ids)
  end

  @doc """
  Returns the number of known peers.
  """
  @spec peer_count() :: non_neg_integer()
  def peer_count do
    GenServer.call(__MODULE__, :peer_count)
  end

  @doc """
  Stops the node manager.
  """
  @spec stop() :: :ok
  def stop do
    GenServer.stop(__MODULE__)
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    data_dir = Keyword.get(opts, :data_dir, System.tmp_dir!())
    File.mkdir_p!(data_dir)

    node_id =
      case Keyword.get(opts, :node_id) do
        nil -> load_or_generate_node_id(data_dir)
        id -> id
      end

    metadata = Keyword.get(opts, :metadata, %{})

    state = %{
      node_id: node_id,
      data_dir: data_dir,
      metadata: metadata,
      peers: %{}
    }

    {:ok, state}
  end

  @impl true
  def handle_call(:node_id, _from, state) do
    {:reply, state.node_id, state}
  end

  @impl true
  def handle_call(:metadata, _from, state) do
    {:reply, state.metadata, state}
  end

  @impl true
  def handle_call({:set_metadata, metadata}, _from, state) do
    {:reply, :ok, %{state | metadata: metadata}}
  end

  @impl true
  def handle_call(:info, _from, state) do
    {:reply, %{id: state.node_id, metadata: state.metadata}, state}
  end

  @impl true
  def handle_call({:add_peer, peer}, _from, state) do
    peers = Map.put(state.peers, peer.id, peer)
    {:reply, :ok, %{state | peers: peers}}
  end

  @impl true
  def handle_call({:remove_peer, peer_id}, _from, state) do
    peers = Map.delete(state.peers, peer_id)
    {:reply, :ok, %{state | peers: peers}}
  end

  @impl true
  def handle_call(:peers, _from, state) do
    {:reply, Map.values(state.peers), state}
  end

  @impl true
  def handle_call({:get_peer, peer_id}, _from, state) do
    {:reply, Map.get(state.peers, peer_id), state}
  end

  @impl true
  def handle_call(:peer_ids, _from, state) do
    {:reply, Map.keys(state.peers), state}
  end

  @impl true
  def handle_call(:peer_count, _from, state) do
    {:reply, map_size(state.peers), state}
  end

  # Private functions

  defp load_or_generate_node_id(data_dir) do
    path = Path.join(data_dir, @node_id_file)

    case File.read(path) do
      {:ok, id} ->
        String.trim(id)

      {:error, :enoent} ->
        id = UUID.uuid4()
        File.write!(path, id)
        id
    end
  end
end
