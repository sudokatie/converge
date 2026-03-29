defmodule Lattice.Storage.WALTest do
  use ExUnit.Case

  alias Lattice.Storage.WAL

  setup do
    tmp_dir = System.tmp_dir!() |> Path.join("lattice_wal_test_#{:rand.uniform(100_000)}")
    File.mkdir_p!(tmp_dir)

    {:ok, _pid} = WAL.start_link(data_dir: tmp_dir)

    on_exit(fn ->
      if Process.whereis(WAL) do
        WAL.close()
        GenServer.stop(WAL)
      end

      File.rm_rf!(tmp_dir)
    end)

    {:ok, data_dir: tmp_dir}
  end

  test "append and replay returns entries" do
    WAL.append({:put, "ns", "key1", :value1})
    WAL.append({:put, "ns", "key2", :value2})
    WAL.append({:delete, "ns", "key1"})

    entries = WAL.replay()
    assert length(entries) == 3
    assert {:put, "ns", "key1", :value1} in entries
    assert {:put, "ns", "key2", :value2} in entries
    assert {:delete, "ns", "key1"} in entries
  end

  test "checkpoint and truncate" do
    WAL.append({:put, "ns", "a", 1})
    WAL.append({:put, "ns", "b", 2})
    WAL.checkpoint()
    WAL.append({:put, "ns", "c", 3})

    # Replay returns entries after checkpoint
    entries = WAL.replay()
    assert length(entries) == 1
    assert {:put, "ns", "c", 3} in entries
  end

  test "empty replay returns empty list" do
    entries = WAL.replay()
    assert entries == []
  end

  test "handles binary values" do
    binary_value = :erlang.term_to_binary(%{complex: "data", list: [1, 2, 3]})
    WAL.append({:put, "ns", "key", binary_value})

    [entry] = WAL.replay()
    assert {:put, "ns", "key", ^binary_value} = entry
  end
end
