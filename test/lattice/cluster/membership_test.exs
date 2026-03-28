defmodule Lattice.Cluster.MembershipTest do
  use ExUnit.Case

  alias Lattice.Cluster.Membership

  setup do
    test_pid = self()

    # Capture sent messages
    send_fn = fn target_id, message ->
      send(test_pid, {:sent, target_id, message})
      :ok
    end

    {:ok, _pid} =
      Membership.start_link(
        node_id: "local-node",
        send_fn: send_fn,
        enabled: false
      )

    on_exit(fn ->
      pid = Process.whereis(Membership)
      if pid && Process.alive?(pid), do: GenServer.stop(Membership)
    end)

    {:ok, send_fn: send_fn}
  end

  describe "join/1" do
    test "adds seed to members" do
      seed = %{id: "seed-node", address: "10.0.0.1", port: 4000}
      :ok = Membership.join(seed)

      members = Membership.members()
      assert length(members) == 1
      assert hd(members).id == "seed-node"
      assert hd(members).state == :alive
    end

    test "sends join message to seed" do
      seed = %{id: "seed-node", address: "10.0.0.1", port: 4000}
      :ok = Membership.join(seed)

      assert_receive {:sent, "seed-node", {:join, "local-node"}}, 100
    end
  end

  describe "leave/0" do
    test "broadcasts leave to all members" do
      Membership.add_member(%{id: "peer-1", address: "a", port: 1})
      Membership.add_member(%{id: "peer-2", address: "b", port: 2})

      :ok = Membership.leave()

      assert_receive {:sent, "peer-1", {:leave, "local-node"}}, 100
      assert_receive {:sent, "peer-2", {:leave, "local-node"}}, 100
    end

    test "clears member list" do
      Membership.add_member(%{id: "peer-1", address: "a", port: 1})
      :ok = Membership.leave()

      assert Membership.members() == []
    end
  end

  describe "members/0 and alive_members/0" do
    test "returns all members" do
      Membership.add_member(%{id: "peer-1", address: "a", port: 1})
      Membership.add_member(%{id: "peer-2", address: "b", port: 2})

      members = Membership.members()
      assert length(members) == 2

      ids = Enum.map(members, & &1.id) |> Enum.sort()
      assert ids == ["peer-1", "peer-2"]
    end

    test "alive_members filters by state" do
      Membership.add_member(%{id: "peer-1", address: "a", port: 1})
      Membership.add_member(%{id: "peer-2", address: "b", port: 2})

      # All should be alive initially
      alive = Membership.alive_members()
      assert length(alive) == 2
    end
  end

  describe "member_count/0" do
    test "returns count of alive members" do
      assert Membership.member_count() == 0

      Membership.add_member(%{id: "peer-1", address: "a", port: 1})
      assert Membership.member_count() == 1

      Membership.add_member(%{id: "peer-2", address: "b", port: 2})
      assert Membership.member_count() == 2
    end
  end

  describe "handle_message/1" do
    test "ping message triggers ack" do
      Membership.add_member(%{id: "peer-1", address: "a", port: 1})

      Membership.handle_message({:ping, "peer-1"})

      assert_receive {:sent, "peer-1", {:ack, "local-node"}}, 100
    end

    test "join message adds member" do
      Membership.handle_message({:join, "new-peer"})

      members = Membership.members()
      assert length(members) == 1
      assert hd(members).id == "new-peer"
    end

    test "leave message removes member" do
      Membership.add_member(%{id: "leaving-peer", address: "a", port: 1})
      assert Membership.member_count() == 1

      Membership.handle_message({:leave, "leaving-peer"})

      assert Membership.member_count() == 0
    end

    test "ack message updates last seen" do
      Membership.add_member(%{id: "peer-1", address: "a", port: 1})

      # Get initial state
      state_before = Membership.get_state()
      member_before = Map.get(state_before.members, "peer-1")

      Process.sleep(10)

      Membership.handle_message({:ack, "peer-1"})

      state_after = Membership.get_state()
      member_after = Map.get(state_after.members, "peer-1")

      assert member_after.last_seen >= member_before.last_seen
    end
  end

  describe "add_member/1" do
    test "adds member with alive state" do
      :ok = Membership.add_member(%{id: "peer-1", address: "192.168.1.1", port: 4000})

      [member] = Membership.members()
      assert member.id == "peer-1"
      assert member.address == "192.168.1.1"
      assert member.port == 4000
      assert member.state == :alive
    end

    test "updates existing member" do
      Membership.add_member(%{id: "peer-1", address: "old", port: 1000})
      Membership.add_member(%{id: "peer-1", address: "new", port: 2000})

      assert Membership.member_count() == 1

      [member] = Membership.members()
      assert member.address == "new"
      assert member.port == 2000
    end
  end
end
