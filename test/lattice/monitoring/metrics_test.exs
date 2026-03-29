defmodule Lattice.Monitoring.MetricsTest do
  use ExUnit.Case, async: true

  alias Lattice.Monitoring.Metrics

  setup do
    {:ok, pid} = Metrics.start_link([])
    on_exit(fn -> Process.alive?(pid) && GenServer.stop(pid) end)
    :ok
  end

  describe "counters" do
    test "increments counter by 1" do
      Metrics.inc(:test_counter)
      assert Metrics.get_counter(:test_counter) == 1
    end

    test "increments counter by amount" do
      Metrics.inc(:test_counter, 5)
      assert Metrics.get_counter(:test_counter) == 5
    end

    test "accumulates increments" do
      Metrics.inc(:test_counter, 3)
      Metrics.inc(:test_counter, 7)
      assert Metrics.get_counter(:test_counter) == 10
    end

    test "returns 0 for unknown counter" do
      assert Metrics.get_counter(:unknown) == 0
    end
  end

  describe "gauges" do
    test "sets gauge value" do
      Metrics.set(:test_gauge, 42.5)
      assert Metrics.get_gauge(:test_gauge) == 42.5
    end

    test "overwrites gauge value" do
      Metrics.set(:test_gauge, 10)
      Metrics.set(:test_gauge, 20)
      assert Metrics.get_gauge(:test_gauge) == 20
    end

    test "returns nil for unknown gauge" do
      assert Metrics.get_gauge(:unknown) == nil
    end
  end

  describe "histograms" do
    test "records observations" do
      Metrics.observe(:test_histogram, 10)
      Metrics.observe(:test_histogram, 20)
      Metrics.observe(:test_histogram, 30)

      metrics = Metrics.get_all()
      assert length(metrics.histograms[:test_histogram]) == 3
    end
  end

  describe "get_all/0" do
    test "returns all metrics" do
      Metrics.inc(:counter1)
      Metrics.set(:gauge1, 100)
      Metrics.observe(:hist1, 5)

      metrics = Metrics.get_all()

      assert is_map(metrics.counters)
      assert is_map(metrics.gauges)
      assert is_map(metrics.histograms)
    end
  end

  describe "reset/0" do
    test "clears all metrics" do
      Metrics.inc(:counter)
      Metrics.set(:gauge, 10)
      Metrics.reset()

      assert Metrics.get_counter(:counter) == 0
      assert Metrics.get_gauge(:gauge) == nil
    end
  end

  describe "report/0" do
    test "returns formatted metrics" do
      Metrics.inc(:sync_operations_total)
      Metrics.set(:data_size_bytes, 1024)

      report = Metrics.report()

      assert is_binary(report)
      assert String.contains?(report, "sync_operations_total")
      assert String.contains?(report, "data_size_bytes")
    end
  end

  describe "convenience functions" do
    test "record_sync increments sync metrics" do
      Metrics.record_sync("myapp")
      assert Metrics.get_counter(:sync_operations_total) == 1
      assert Metrics.get_counter(:sync_operations_myapp) == 1
    end

    test "record_merge_conflict increments conflict metrics" do
      Metrics.record_merge_conflict(:g_counter)
      assert Metrics.get_counter(:merge_conflicts_total) == 1
      assert Metrics.get_counter(:merge_conflicts_g_counter) == 1
    end

    test "update_data_size sets gauge" do
      Metrics.update_data_size("namespace1", 2048)
      assert Metrics.get_gauge(:data_size_bytes_namespace1) == 2048
    end

    test "record_bytes_sent increments network counter" do
      Metrics.record_bytes_sent(100)
      Metrics.record_bytes_sent(50)
      assert Metrics.get_counter(:network_bytes_sent) == 150
    end

    test "record_bytes_received increments network counter" do
      Metrics.record_bytes_received(200)
      assert Metrics.get_counter(:network_bytes_received) == 200
    end

    test "record_membership_change increments membership metrics" do
      Metrics.record_membership_change(:join)
      Metrics.record_membership_change(:leave)
      assert Metrics.get_counter(:membership_changes_total) == 2
      assert Metrics.get_counter(:membership_join) == 1
      assert Metrics.get_counter(:membership_leave) == 1
    end
  end
end
