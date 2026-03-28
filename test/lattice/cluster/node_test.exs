defmodule Lattice.Cluster.NodeTest do
  use ExUnit.Case

  alias Lattice.Cluster.Node

  setup do
    tmp_dir = System.tmp_dir!() |> Path.join("lattice_node_test_#{:rand.uniform(100000)}")
    File.mkdir_p!(tmp_dir)

    {:ok, _pid} = Node.start_link(data_dir: tmp_dir)

    on_exit(fn ->
      if Process.whereis(Node), do: GenServer.stop(Node)
      File.rm_rf!(tmp_dir)
    end)

    {:ok, data_dir: tmp_dir}
  end

  describe "node_id/0" do
    test "returns a UUID string" do
      id = Node.node_id()
      assert is_binary(id)
      assert String.length(id) == 36
    end

    test "id persists across restarts", %{data_dir: data_dir} do
      id1 = Node.node_id()
      Node.stop()

      {:ok, _pid} = Node.start_link(data_dir: data_dir)
      id2 = Node.node_id()

      assert id1 == id2
    end
  end

  describe "metadata/0 and set_metadata/1" do
    test "metadata starts empty by default" do
      assert Node.metadata() == %{}
    end

    test "can set and get metadata" do
      Node.set_metadata(%{address: "127.0.0.1", port: 4000})

      meta = Node.metadata()
      assert meta.address == "127.0.0.1"
      assert meta.port == 4000
    end
  end

  describe "info/0" do
    test "returns id and metadata together" do
      Node.set_metadata(%{role: :leader})

      info = Node.info()
      assert is_binary(info.id)
      assert info.metadata.role == :leader
    end
  end

  describe "peer management" do
    test "add_peer and peers returns all peers" do
      peer1 = %{id: "peer-1", address: "192.168.1.1", port: 4000, metadata: %{}}
      peer2 = %{id: "peer-2", address: "192.168.1.2", port: 4000, metadata: %{}}

      Node.add_peer(peer1)
      Node.add_peer(peer2)

      peers = Node.peers()
      assert length(peers) == 2

      ids = Enum.map(peers, & &1.id) |> Enum.sort()
      assert ids == ["peer-1", "peer-2"]
    end

    test "remove_peer removes by ID" do
      peer = %{id: "peer-1", address: "192.168.1.1", port: 4000, metadata: %{}}
      Node.add_peer(peer)
      assert Node.peer_count() == 1

      Node.remove_peer("peer-1")
      assert Node.peer_count() == 0
    end

    test "get_peer returns specific peer" do
      peer = %{id: "peer-1", address: "10.0.0.1", port: 5000, metadata: %{region: "us-west"}}
      Node.add_peer(peer)

      result = Node.get_peer("peer-1")
      assert result.address == "10.0.0.1"
      assert result.metadata.region == "us-west"
    end

    test "get_peer returns nil for unknown peer" do
      assert Node.get_peer("nonexistent") == nil
    end

    test "peer_ids returns just IDs" do
      Node.add_peer(%{id: "a", address: "", port: 0, metadata: %{}})
      Node.add_peer(%{id: "b", address: "", port: 0, metadata: %{}})

      ids = Node.peer_ids() |> Enum.sort()
      assert ids == ["a", "b"]
    end

    test "peer_count returns count" do
      assert Node.peer_count() == 0

      Node.add_peer(%{id: "a", address: "", port: 0, metadata: %{}})
      assert Node.peer_count() == 1

      Node.add_peer(%{id: "b", address: "", port: 0, metadata: %{}})
      assert Node.peer_count() == 2
    end

    test "adding same peer ID updates peer" do
      Node.add_peer(%{id: "peer-1", address: "old.addr", port: 1000, metadata: %{}})
      Node.add_peer(%{id: "peer-1", address: "new.addr", port: 2000, metadata: %{}})

      assert Node.peer_count() == 1

      peer = Node.get_peer("peer-1")
      assert peer.address == "new.addr"
      assert peer.port == 2000
    end
  end

  describe "explicit node_id option" do
    test "uses provided node_id instead of generating", %{data_dir: _data_dir} do
      Node.stop()

      tmp = System.tmp_dir!() |> Path.join("lattice_explicit_#{:rand.uniform(100000)}")
      File.mkdir_p!(tmp)

      {:ok, _} = Node.start_link(node_id: "my-custom-id", data_dir: tmp)

      assert Node.node_id() == "my-custom-id"

      File.rm_rf!(tmp)
    end
  end
end
