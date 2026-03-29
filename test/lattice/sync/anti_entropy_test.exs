defmodule Lattice.Sync.AntiEntropyTest do
  use ExUnit.Case

  alias Lattice.Sync.AntiEntropy

  setup do
    # Start with timer disabled for deterministic tests
    {:ok, pid} = AntiEntropy.start_link(node_id: "test-node", start_timer: false)
    on_exit(fn -> if Process.alive?(pid), do: GenServer.stop(pid) end)
    {:ok, pid: pid}
  end

  describe "start_link/1" do
    test "starts with default options" do
      # Already started in setup
      state = AntiEntropy.get_state()
      assert state.node_id == "test-node"
      assert state.interval_ms == 5_000
      assert state.peers == []
    end
  end

  describe "update_merkle/3" do
    test "adds key to merkle tree" do
      AntiEntropy.update_merkle("default", "key1", "hash1")

      # Small delay for cast to process
      Process.sleep(10)

      root = AntiEntropy.get_merkle_root("default")
      assert is_binary(root)
      assert byte_size(root) == 32
    end

    test "different values produce different roots" do
      AntiEntropy.update_merkle("ns1", "key1", "value1")
      Process.sleep(10)
      root1 = AntiEntropy.get_merkle_root("ns1")

      AntiEntropy.update_merkle("ns1", "key1", "value2")
      Process.sleep(10)
      root2 = AntiEntropy.get_merkle_root("ns1")

      assert root1 != root2
    end
  end

  describe "remove_from_merkle/2" do
    test "removes key from tree" do
      AntiEntropy.update_merkle("default", "key1", "hash1")
      AntiEntropy.update_merkle("default", "key2", "hash2")
      Process.sleep(10)

      root_before = AntiEntropy.get_merkle_root("default")

      AntiEntropy.remove_from_merkle("default", "key1")
      Process.sleep(10)

      root_after = AntiEntropy.get_merkle_root("default")

      assert root_before != root_after
    end
  end

  describe "set_peers/1" do
    test "updates peer list" do
      AntiEntropy.set_peers(["peer1", "peer2"])
      Process.sleep(10)

      state = AntiEntropy.get_state()
      assert state.peers == ["peer1", "peer2"]
    end
  end

  describe "sync_now/0" do
    test "triggers sync and updates last_sync" do
      state_before = AntiEntropy.get_state()
      assert state_before.last_sync == nil

      AntiEntropy.sync_now()
      Process.sleep(10)

      state_after = AntiEntropy.get_state()
      assert state_after.last_sync != nil
    end
  end

  describe "sync_now/1" do
    test "syncs specific namespace" do
      AntiEntropy.update_merkle("ns1", "k1", "v1")
      Process.sleep(10)

      AntiEntropy.sync_now("ns1")
      Process.sleep(10)

      state = AntiEntropy.get_state()
      # Only full sync updates last_sync
      assert state.last_sync == nil
    end
  end

  describe "handle_sync_request/3" do
    test "returns empty when roots match" do
      AntiEntropy.update_merkle("ns", "key1", "value1")
      Process.sleep(10)

      our_root = AntiEntropy.get_merkle_root("ns")

      {:ok, keys} = AntiEntropy.handle_sync_request("ns", our_root, "peer-1")
      assert keys == []
    end

    test "returns keys when roots differ" do
      AntiEntropy.update_merkle("ns", "key1", "value1")
      AntiEntropy.update_merkle("ns", "key2", "value2")
      Process.sleep(10)

      different_root = :crypto.hash(:sha256, "different")

      {:ok, keys} = AntiEntropy.handle_sync_request("ns", different_root, "peer-1")
      assert "key1" in keys
      assert "key2" in keys
    end
  end

  describe "get_merkle_root/1" do
    test "returns consistent root for same data" do
      AntiEntropy.update_merkle("ns", "a", "1")
      AntiEntropy.update_merkle("ns", "b", "2")
      Process.sleep(10)

      root1 = AntiEntropy.get_merkle_root("ns")
      root2 = AntiEntropy.get_merkle_root("ns")

      assert root1 == root2
    end

    test "empty namespace has consistent empty root" do
      root1 = AntiEntropy.get_merkle_root("empty1")
      root2 = AntiEntropy.get_merkle_root("empty2")

      assert root1 == root2
    end
  end
end
