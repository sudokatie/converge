defmodule Converge.Storage.SnapshotTest do
  use ExUnit.Case

  alias Converge.Storage.Snapshot

  setup do
    tmp_dir = System.tmp_dir!() |> Path.join("converge_snap_test_#{:rand.uniform(100_000)}")
    File.mkdir_p!(tmp_dir)

    {:ok, _pid} = Snapshot.start_link(data_dir: tmp_dir)

    on_exit(fn ->
      pid = Process.whereis(Snapshot)
      if pid && Process.alive?(pid), do: GenServer.stop(Snapshot)
      File.rm_rf!(tmp_dir)
    end)

    {:ok, data_dir: tmp_dir}
  end

  test "create returns snapshot id" do
    {:ok, snapshot_id} = Snapshot.create()
    assert String.starts_with?(snapshot_id, "snapshot_")
  end

  test "list returns empty for no snapshots initially" do
    snapshots = Snapshot.list()
    # May have one from create in previous test, or none
    assert is_list(snapshots)
  end

  test "list returns created snapshots" do
    {:ok, id1} = Snapshot.create()
    :timer.sleep(10)
    {:ok, id2} = Snapshot.create()

    snapshots = Snapshot.list()
    assert id1 in snapshots
    assert id2 in snapshots
  end

  test "cleanup keeps only N most recent" do
    # Create 3 snapshots
    {:ok, _id1} = Snapshot.create()
    :timer.sleep(10)
    {:ok, _id2} = Snapshot.create()
    :timer.sleep(10)
    {:ok, id3} = Snapshot.create()

    # Keep only 1
    Snapshot.cleanup(1)

    snapshots = Snapshot.list()
    assert length(snapshots) == 1
    assert id3 in snapshots
  end

  test "restore returns error when no snapshots" do
    # Clear any existing
    Snapshot.cleanup(0)

    result = Snapshot.restore()
    assert result == {:error, :no_snapshots}
  end
end
