defmodule Lattice.Sync.AntiEntropy do
  @moduledoc """
  Anti-entropy process for periodic synchronization.

  Periodically compares Merkle tree roots with peers and requests
  missing data to achieve convergence.
  """

  use GenServer

  alias Lattice.Sync.{Merkle, Protocol}

  @default_interval_ms 5_000

  @type state :: %{
          interval_ms: pos_integer(),
          merkle_trees: %{String.t() => Merkle.t()},
          peers: [String.t()],
          node_id: String.t(),
          timer_ref: reference() | nil,
          sync_handler: module() | nil,
          last_sync: integer() | nil
        }

  # Client API

  @doc """
  Starts the anti-entropy process.

  Options:
  - :interval_ms - sync interval (default 5000)
  - :node_id - this node's identifier
  - :sync_handler - module that handles sync messages
  """
  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Triggers immediate sync for all namespaces.
  """
  @spec sync_now() :: :ok
  def sync_now do
    GenServer.cast(__MODULE__, :sync_now)
  end

  @doc """
  Triggers immediate sync for a specific namespace.
  """
  @spec sync_now(String.t()) :: :ok
  def sync_now(namespace) do
    GenServer.cast(__MODULE__, {:sync_now, namespace})
  end

  @doc """
  Updates the Merkle tree for a namespace when data changes.
  """
  @spec update_merkle(String.t(), String.t(), binary()) :: :ok
  def update_merkle(namespace, key, value_hash) do
    GenServer.cast(__MODULE__, {:update_merkle, namespace, key, value_hash})
  end

  @doc """
  Removes a key from the Merkle tree.
  """
  @spec remove_from_merkle(String.t(), String.t()) :: :ok
  def remove_from_merkle(namespace, key) do
    GenServer.cast(__MODULE__, {:remove_from_merkle, namespace, key})
  end

  @doc """
  Updates the peer list.
  """
  @spec set_peers([String.t()]) :: :ok
  def set_peers(peers) do
    GenServer.cast(__MODULE__, {:set_peers, peers})
  end

  @doc """
  Returns current sync state for debugging.
  """
  @spec get_state() :: state()
  def get_state do
    GenServer.call(__MODULE__, :get_state)
  end

  @doc """
  Returns the Merkle root for a namespace.
  """
  @spec get_merkle_root(String.t()) :: binary()
  def get_merkle_root(namespace) do
    GenServer.call(__MODULE__, {:get_merkle_root, namespace})
  end

  @doc """
  Handles an incoming sync request from a peer.
  Returns the diff keys that the peer needs.
  """
  @spec handle_sync_request(String.t(), binary(), String.t()) :: {:ok, [String.t()]}
  def handle_sync_request(namespace, peer_root, _from_node) do
    GenServer.call(__MODULE__, {:handle_sync_request, namespace, peer_root})
  end

  @doc """
  Stops the anti-entropy process.
  """
  @spec stop() :: :ok
  def stop do
    GenServer.stop(__MODULE__)
  end

  # Server callbacks

  @impl true
  def init(opts) do
    interval = Keyword.get(opts, :interval_ms, @default_interval_ms)
    node_id = Keyword.get(opts, :node_id, UUID.uuid4())
    sync_handler = Keyword.get(opts, :sync_handler, nil)
    start_timer = Keyword.get(opts, :start_timer, true)

    state = %{
      interval_ms: interval,
      merkle_trees: %{},
      peers: [],
      node_id: node_id,
      timer_ref: nil,
      sync_handler: sync_handler,
      last_sync: nil
    }

    state =
      if start_timer do
        schedule_sync(state)
      else
        state
      end

    {:ok, state}
  end

  @impl true
  def handle_cast(:sync_now, state) do
    new_state = do_sync(state)
    {:noreply, new_state}
  end

  @impl true
  def handle_cast({:sync_now, namespace}, state) do
    new_state = do_sync_namespace(state, namespace)
    {:noreply, new_state}
  end

  @impl true
  def handle_cast({:update_merkle, namespace, key, value_hash}, state) do
    tree = get_or_create_tree(state, namespace)
    tree = Merkle.insert(tree, key, value_hash)
    trees = Map.put(state.merkle_trees, namespace, tree)
    {:noreply, %{state | merkle_trees: trees}}
  end

  @impl true
  def handle_cast({:remove_from_merkle, namespace, key}, state) do
    tree = get_or_create_tree(state, namespace)
    tree = Merkle.remove(tree, key)
    trees = Map.put(state.merkle_trees, namespace, tree)
    {:noreply, %{state | merkle_trees: trees}}
  end

  @impl true
  def handle_cast({:set_peers, peers}, state) do
    {:noreply, %{state | peers: peers}}
  end

  @impl true
  def handle_call(:get_state, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_call({:get_merkle_root, namespace}, _from, state) do
    tree = get_or_create_tree(state, namespace)
    {root, tree} = Merkle.root_hash(tree)
    trees = Map.put(state.merkle_trees, namespace, tree)
    {:reply, root, %{state | merkle_trees: trees}}
  end

  @impl true
  def handle_call({:handle_sync_request, namespace, peer_root}, _from, state) do
    tree = get_or_create_tree(state, namespace)
    {our_root, tree} = Merkle.root_hash(tree)

    diff_keys =
      if our_root == peer_root do
        []
      else
        # Return all keys - peer will compare and request what they need
        Merkle.keys(tree)
      end

    trees = Map.put(state.merkle_trees, namespace, tree)
    {:reply, {:ok, diff_keys}, %{state | merkle_trees: trees}}
  end

  @impl true
  def handle_info(:sync, state) do
    new_state =
      state
      |> do_sync()
      |> schedule_sync()

    {:noreply, new_state}
  end

  # Private functions

  defp schedule_sync(%{interval_ms: interval} = state) do
    if state.timer_ref do
      Process.cancel_timer(state.timer_ref)
    end

    ref = Process.send_after(self(), :sync, interval)
    %{state | timer_ref: ref}
  end

  defp do_sync(state) do
    namespaces = Map.keys(state.merkle_trees)

    Enum.reduce(namespaces, state, fn namespace, acc_state ->
      do_sync_namespace(acc_state, namespace)
    end)
    |> Map.put(:last_sync, System.monotonic_time(:millisecond))
  end

  defp do_sync_namespace(state, namespace) do
    tree = get_or_create_tree(state, namespace)
    {root, tree} = Merkle.root_hash(tree)
    trees = Map.put(state.merkle_trees, namespace, tree)

    # Send sync request to all peers
    if state.sync_handler do
      msg = Protocol.sync_request(namespace, root, state.node_id)

      Enum.each(state.peers, fn peer ->
        state.sync_handler.send_to_peer(peer, msg)
      end)
    end

    %{state | merkle_trees: trees}
  end

  defp get_or_create_tree(state, namespace) do
    Map.get(state.merkle_trees, namespace, Merkle.new())
  end
end
