defmodule Lattice.ConsistencyTest do
  use ExUnit.Case, async: false

  alias Lattice.Consistency

  setup do
    # Start store for quorum operations
    tmp_dir = System.tmp_dir!() |> Path.join("consistency_test_#{:rand.uniform(100_000)}")
    File.mkdir_p!(tmp_dir)
    {:ok, store} = Lattice.Storage.Store.start_link(data_dir: tmp_dir)
    {:ok, membership} = Lattice.Sync.Membership.start_link(enabled: false)
    {:ok, consistency} = Consistency.start_link([])

    on_exit(fn ->
      Process.alive?(consistency) && GenServer.stop(consistency)
      Process.alive?(membership) && GenServer.stop(membership)
      Process.alive?(store) && GenServer.stop(store)
      File.rm_rf!(tmp_dir)
    end)

    :ok
  end

  describe "default level" do
    test "defaults to :eventual" do
      assert Consistency.get_default_level() == :eventual
    end

    test "can set default level" do
      Consistency.set_default_level(:session)
      assert Consistency.get_default_level() == :session

      Consistency.set_default_level(:quorum)
      assert Consistency.get_default_level() == :quorum

      Consistency.set_default_level(:eventual)
      assert Consistency.get_default_level() == :eventual
    end
  end

  describe "sessions" do
    test "creates a new session" do
      session_id = Consistency.new_session()
      assert is_binary(session_id)
      assert String.length(session_id) > 0
    end

    test "sessions have unique IDs" do
      s1 = Consistency.new_session()
      s2 = Consistency.new_session()
      assert s1 != s2
    end

    test "records writes in session" do
      session_id = Consistency.new_session()
      assert :ok = Consistency.record_write(session_id, "ns", "key")
    end

    test "check_session_read returns :ok for session keys" do
      session_id = Consistency.new_session()
      Consistency.record_write(session_id, "ns", "key")
      assert Consistency.check_session_read(session_id, "ns", "key") == :ok
    end

    test "check_session_read returns :ok for unknown session" do
      assert Consistency.check_session_read("unknown", "ns", "key") == :ok
    end
  end

  describe "quorum_read/2" do
    test "reads local value when no cluster members" do
      # Store a value
      Lattice.Storage.Store.put("ns", "key", "value")

      {:ok, result} = Consistency.quorum_read("ns", "key")
      assert result == "value"
    end

    test "returns nil for missing key" do
      {:ok, result} = Consistency.quorum_read("ns", "nonexistent")
      assert result == nil
    end
  end

  describe "quorum_write/3" do
    test "writes value locally when no cluster members" do
      assert :ok = Consistency.quorum_write("ns", "key", "value")
      assert Lattice.Storage.Store.get("ns", "key") == "value"
    end
  end

  describe "cleanup_sessions/0" do
    test "cleans up sessions" do
      _session_id = Consistency.new_session()
      assert :ok = Consistency.cleanup_sessions()
    end
  end
end
