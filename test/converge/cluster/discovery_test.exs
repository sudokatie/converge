defmodule Converge.Cluster.DiscoveryTest do
  use ExUnit.Case

  alias Converge.Cluster.Discovery

  setup do
    # Start with UDP disabled for unit tests
    {:ok, _pid} =
      Discovery.start_link(
        node_id: "test-node",
        address: "127.0.0.1",
        port: 4000,
        enabled: false
      )

    on_exit(fn ->
      pid = Process.whereis(Discovery)
      if pid && Process.alive?(pid), do: GenServer.stop(Discovery)
    end)

    :ok
  end

  describe "start_link/1" do
    test "starts with provided options" do
      state = Discovery.get_state()

      assert state.node_id == "test-node"
      assert state.address == "127.0.0.1"
      assert state.port == 4000
      assert state.enabled == false
    end
  end

  describe "announce/0" do
    test "does not crash when socket is nil" do
      # Should handle gracefully when UDP is disabled
      assert :ok = Discovery.announce()
    end
  end

  describe "handle_discovery/1" do
    test "processes valid peer announcement" do
      test_pid = self()

      # Restart with custom callback
      Discovery.stop()

      {:ok, _} =
        Discovery.start_link(
          node_id: "local-node",
          enabled: false,
          on_discover: fn peer -> send(test_pid, {:discovered, peer}) end
        )

      # Create announcement from another node
      announcement = %{
        service: "_converge._udp.local",
        node_id: "remote-node",
        address: "192.168.1.100",
        port: 4001,
        metadata: %{region: "us-east"},
        timestamp: System.system_time(:millisecond)
      }

      message = :erlang.term_to_binary(announcement)
      Discovery.handle_discovery(message)

      assert_receive {:discovered, peer}, 100
      assert peer.id == "remote-node"
      assert peer.address == "192.168.1.100"
      assert peer.port == 4001
      assert peer.metadata.region == "us-east"
    end

    test "ignores own announcements" do
      test_pid = self()

      Discovery.stop()

      {:ok, _} =
        Discovery.start_link(
          node_id: "same-node",
          enabled: false,
          on_discover: fn peer -> send(test_pid, {:discovered, peer}) end
        )

      # Announcement from same node ID
      announcement = %{
        service: "_converge._udp.local",
        node_id: "same-node",
        address: "127.0.0.1",
        port: 4000,
        metadata: %{},
        timestamp: System.system_time(:millisecond)
      }

      message = :erlang.term_to_binary(announcement)
      Discovery.handle_discovery(message)

      refute_receive {:discovered, _}, 50
    end

    test "ignores invalid messages" do
      test_pid = self()

      Discovery.stop()

      {:ok, _} =
        Discovery.start_link(
          node_id: "local-node",
          enabled: false,
          on_discover: fn peer -> send(test_pid, {:discovered, peer}) end
        )

      # Invalid binary
      Discovery.handle_discovery(<<1, 2, 3, 4>>)
      refute_receive {:discovered, _}, 50

      # Different service name
      other_service = %{
        service: "_other._tcp.local",
        node_id: "other-node",
        address: "10.0.0.1",
        port: 5000,
        metadata: %{},
        timestamp: 0
      }

      Discovery.handle_discovery(:erlang.term_to_binary(other_service))
      refute_receive {:discovered, _}, 50
    end

    test "handles multiple peer discoveries" do
      test_pid = self()

      Discovery.stop()

      {:ok, _} =
        Discovery.start_link(
          node_id: "local",
          enabled: false,
          on_discover: fn peer -> send(test_pid, {:discovered, peer}) end
        )

      for i <- 1..3 do
        announcement = %{
          service: "_converge._udp.local",
          node_id: "peer-#{i}",
          address: "10.0.0.#{i}",
          port: 4000 + i,
          metadata: %{},
          timestamp: System.system_time(:millisecond)
        }

        Discovery.handle_discovery(:erlang.term_to_binary(announcement))
      end

      assert_receive {:discovered, %{id: "peer-1"}}, 100
      assert_receive {:discovered, %{id: "peer-2"}}, 100
      assert_receive {:discovered, %{id: "peer-3"}}, 100
    end
  end
end
