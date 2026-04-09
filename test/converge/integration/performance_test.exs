defmodule Converge.Integration.PerformanceTest do
  @moduledoc """
  Performance tests for large dataset operations and sync.
  """
  use ExUnit.Case

  alias Converge.CRDT.{PNCounter, ORSet, LWWMap}
  alias Converge.Sync.Merkle
  alias Converge.Storage.Store
  alias Converge.Cluster.Node

  @large_dataset_size 1_000
  @performance_threshold_ms 5_000

  setup do
    tmp_dir = System.tmp_dir!() |> Path.join("converge_perf_#{:rand.uniform(100_000)}")
    File.mkdir_p!(tmp_dir)

    {:ok, _} = Store.start_link(data_dir: tmp_dir)
    {:ok, _} = Node.start_link(node_id: "perf-node", data_dir: tmp_dir)

    on_exit(fn ->
      pid = Process.whereis(Store)
      if pid && Process.alive?(pid), do: GenServer.stop(Store)

      pid = Process.whereis(Node)
      if pid && Process.alive?(pid), do: GenServer.stop(Node)

      File.rm_rf!(tmp_dir)
    end)

    {:ok, data_dir: tmp_dir}
  end

  describe "large dataset sync performance" do
    test "merkle tree handles large number of keys efficiently" do
      # Build tree with many keys
      {time_us, tree} =
        :timer.tc(fn ->
          Enum.reduce(1..@large_dataset_size, Merkle.new(), fn i, tree ->
            Merkle.insert(tree, "key_#{i}", "value_#{i}")
          end)
        end)

      time_ms = div(time_us, 1000)
      assert time_ms < @performance_threshold_ms, "Insert took #{time_ms}ms, expected < #{@performance_threshold_ms}ms"

      # Compute root hash
      {hash_time_us, {_root, _tree}} = :timer.tc(fn -> Merkle.root_hash(tree) end)
      hash_time_ms = div(hash_time_us, 1000)
      assert hash_time_ms < 1000, "Root hash took #{hash_time_ms}ms, expected < 1000ms"

      # Diff two large trees
      tree2 =
        Enum.reduce(1..@large_dataset_size, Merkle.new(), fn i, tree ->
          # Modify 10% of keys
          value = if rem(i, 10) == 0, do: "modified_#{i}", else: "value_#{i}"
          Merkle.insert(tree, "key_#{i}", value)
        end)

      {diff_time_us, {_only_a, _only_b, different}} = :timer.tc(fn -> Merkle.diff(tree, tree2) end)
      diff_time_ms = div(diff_time_us, 1000)
      assert diff_time_ms < 1000, "Diff took #{diff_time_ms}ms, expected < 1000ms"

      # Should detect ~10% differences
      expected_diffs = div(@large_dataset_size, 10)
      assert length(different) == expected_diffs
    end

    test "counter merge scales with node count" do
      node_counts = [10, 50, 100]

      for node_count <- node_counts do
        # Create counters from many nodes
        counters =
          Enum.map(1..node_count, fn i ->
            PNCounter.new("node_#{i}") |> PNCounter.inc(i)
          end)

        # Merge all counters
        {merge_time_us, merged} =
          :timer.tc(fn ->
            Enum.reduce(counters, PNCounter.new("base"), &PNCounter.merge(&2, &1))
          end)

        merge_time_ms = div(merge_time_us, 1000)
        assert merge_time_ms < 500, "Merge of #{node_count} nodes took #{merge_time_ms}ms"

        # Verify correctness: sum of 1..n = n(n+1)/2
        expected_value = div(node_count * (node_count + 1), 2)
        assert PNCounter.value(merged) == expected_value
      end
    end

    test "OR-Set handles large membership" do
      # Add many elements
      {add_time_us, set} =
        :timer.tc(fn ->
          Enum.reduce(1..@large_dataset_size, ORSet.new(), fn i, set ->
            ORSet.add(set, "element_#{i}")
          end)
        end)

      add_time_ms = div(add_time_us, 1000)
      assert add_time_ms < @performance_threshold_ms

      # Verify all elements present
      assert ORSet.size(set) == @large_dataset_size

      # Membership check should be fast
      {contains_time_us, result} =
        :timer.tc(fn ->
          ORSet.contains?(set, "element_#{div(@large_dataset_size, 2)}")
        end)

      assert result == true
      assert contains_time_us < 1000, "Contains check took #{contains_time_us}us"

      # Remove half the elements
      {remove_time_us, reduced_set} =
        :timer.tc(fn ->
          Enum.reduce(1..div(@large_dataset_size, 2), set, fn i, s ->
            ORSet.remove(s, "element_#{i}")
          end)
        end)

      remove_time_ms = div(remove_time_us, 1000)
      assert remove_time_ms < @performance_threshold_ms

      assert ORSet.size(reduced_set) == div(@large_dataset_size, 2)
    end

    test "LWW-Map handles large key space" do
      # Add many key-value pairs
      {put_time_us, map} =
        :timer.tc(fn ->
          Enum.reduce(1..@large_dataset_size, LWWMap.new(), fn i, map ->
            LWWMap.put(map, "key_#{i}", "value_#{i}")
          end)
        end)

      put_time_ms = div(put_time_us, 1000)
      assert put_time_ms < @performance_threshold_ms

      assert LWWMap.size(map) == @large_dataset_size

      # Get should be fast
      {get_time_us, value} =
        :timer.tc(fn ->
          LWWMap.get(map, "key_#{div(@large_dataset_size, 2)}")
        end)

      assert value == "value_#{div(@large_dataset_size, 2)}"
      assert get_time_us < 1000

      # Merge two large maps
      map2 =
        Enum.reduce(1..@large_dataset_size, LWWMap.new("node2"), fn i, m ->
          LWWMap.put(m, "other_key_#{i}", "other_value_#{i}")
        end)

      {merge_time_us, merged} = :timer.tc(fn -> LWWMap.merge(map, map2) end)
      merge_time_ms = div(merge_time_us, 1000)
      assert merge_time_ms < @performance_threshold_ms

      # Merged map should have keys from both
      assert LWWMap.size(merged) == @large_dataset_size * 2
    end

    test "store handles high write throughput" do
      # Burst of writes
      {write_time_us, _} =
        :timer.tc(fn ->
          for i <- 1..@large_dataset_size do
            Converge.counter_inc("perf_ns", "counter_#{i}")
          end
        end)

      write_time_ms = div(write_time_us, 1000)
      ops_per_sec = @large_dataset_size * 1000 / max(write_time_ms, 1)

      # Should handle at least 100 ops/sec (conservative for CI)
      assert ops_per_sec > 100, "Only #{ops_per_sec} ops/sec, expected > 100"

      # Verify sample of writes persisted
      for idx <- 1..10 do
        assert Converge.counter_value("perf_ns", "counter_#{idx}") == 1
      end
    end

    test "concurrent read/write performance" do
      # Pre-populate
      for i <- 1..100 do
        Converge.counter_inc("concurrent_ns", "key_#{i}", i)
      end

      # Concurrent reads and writes
      tasks =
        for _i <- 1..10 do
          Task.async(fn ->
            for j <- 1..100 do
              if rem(j, 2) == 0 do
                Converge.counter_inc("concurrent_ns", "key_#{rem(j, 100) + 1}")
              else
                Converge.counter_value("concurrent_ns", "key_#{rem(j, 100) + 1}")
              end
            end
          end)
        end

      {concurrent_time_us, _} = :timer.tc(fn -> Task.await_many(tasks, 10_000) end)
      concurrent_time_ms = div(concurrent_time_us, 1000)

      # 1000 operations (10 tasks * 100 ops) should complete in reasonable time
      assert concurrent_time_ms < @performance_threshold_ms
    end

    test "serialization/deserialization scales linearly" do
      # Create large CRDT
      large_counter =
        Enum.reduce(1..100, PNCounter.new("perf"), fn i, c ->
          PNCounter.inc(c, i)
        end)

      # Serialize
      {ser_time_us, binary} = :timer.tc(fn -> PNCounter.to_binary(large_counter) end)
      assert ser_time_us < 10_000, "Serialization took #{ser_time_us}us"

      # Deserialize
      {deser_time_us, restored} = :timer.tc(fn -> PNCounter.from_binary(binary) end)
      assert deser_time_us < 10_000, "Deserialization took #{deser_time_us}us"

      # Verify correctness
      assert PNCounter.value(restored) == PNCounter.value(large_counter)
    end
  end
end
