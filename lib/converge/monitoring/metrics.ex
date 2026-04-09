defmodule Converge.Monitoring.Metrics do
  @moduledoc """
  Metrics collection for Converge.

  Tracks:
  - Sync operations per second
  - Merge conflicts per type
  - Data size per namespace
  - Network bytes in/out
  - Node membership changes
  """

  use GenServer

  @type metric_name :: atom()
  @type metric_value :: number()

  @type state :: %{
          counters: %{metric_name() => integer()},
          gauges: %{metric_name() => number()},
          histograms: %{metric_name() => [number()]},
          start_time: integer()
        }

  # Client API

  @doc """
  Starts the metrics collector.
  """
  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Increments a counter metric.
  """
  @spec inc(metric_name()) :: :ok
  def inc(name) do
    inc(name, 1)
  end

  @spec inc(metric_name(), integer()) :: :ok
  def inc(name, amount) when is_integer(amount) do
    if Process.whereis(__MODULE__) do
      GenServer.cast(__MODULE__, {:inc, name, amount})
    end

    :ok
  end

  @doc """
  Sets a gauge metric value.
  """
  @spec set(metric_name(), number()) :: :ok
  def set(name, value) when is_number(value) do
    if Process.whereis(__MODULE__) do
      GenServer.cast(__MODULE__, {:set, name, value})
    end

    :ok
  end

  @doc """
  Records a value in a histogram.
  """
  @spec observe(metric_name(), number()) :: :ok
  def observe(name, value) when is_number(value) do
    if Process.whereis(__MODULE__) do
      GenServer.cast(__MODULE__, {:observe, name, value})
    end

    :ok
  end

  @doc """
  Gets all metrics as a map.
  """
  @spec get_all() :: map()
  def get_all do
    if Process.whereis(__MODULE__) do
      GenServer.call(__MODULE__, :get_all)
    else
      %{counters: %{}, gauges: %{}, histograms: %{}}
    end
  end

  @doc """
  Gets a specific counter value.
  """
  @spec get_counter(metric_name()) :: integer()
  def get_counter(name) do
    if Process.whereis(__MODULE__) do
      GenServer.call(__MODULE__, {:get_counter, name})
    else
      0
    end
  end

  @doc """
  Gets a specific gauge value.
  """
  @spec get_gauge(metric_name()) :: number() | nil
  def get_gauge(name) do
    if Process.whereis(__MODULE__) do
      GenServer.call(__MODULE__, {:get_gauge, name})
    else
      nil
    end
  end

  @doc """
  Resets all metrics.
  """
  @spec reset() :: :ok
  def reset do
    if Process.whereis(__MODULE__) do
      GenServer.cast(__MODULE__, :reset)
    end

    :ok
  end

  @doc """
  Returns formatted metrics report.
  """
  @spec report() :: String.t()
  def report do
    metrics = get_all()
    uptime = System.monotonic_time(:second) - (metrics[:start_time] || 0)

    lines = [
      "# Converge Metrics",
      "uptime_seconds #{uptime}",
      "",
      "# Counters"
    ]

    counter_lines =
      Enum.map(metrics.counters || %{}, fn {name, value} ->
        "#{name} #{value}"
      end)

    gauge_lines =
      Enum.map(metrics.gauges || %{}, fn {name, value} ->
        "#{name} #{value}"
      end)

    histogram_lines =
      Enum.flat_map(metrics.histograms || %{}, fn {name, values} ->
        if length(values) > 0 do
          avg = Enum.sum(values) / length(values)
          min = Enum.min(values)
          max = Enum.max(values)

          [
            "#{name}_count #{length(values)}",
            "#{name}_avg #{Float.round(avg, 3)}",
            "#{name}_min #{min}",
            "#{name}_max #{max}"
          ]
        else
          []
        end
      end)

    all_lines =
      lines ++
        counter_lines ++
        ["", "# Gauges"] ++
        gauge_lines ++
        ["", "# Histograms"] ++
        histogram_lines

    Enum.join(all_lines, "\n")
  end

  # Convenience functions for common metrics

  @doc """
  Records a sync operation.
  """
  @spec record_sync(String.t()) :: :ok
  def record_sync(namespace) do
    inc(:sync_operations_total)
    inc(:"sync_operations_#{namespace}")
    :ok
  end

  @doc """
  Records a merge conflict.
  """
  @spec record_merge_conflict(atom()) :: :ok
  def record_merge_conflict(crdt_type) do
    inc(:merge_conflicts_total)
    inc(:"merge_conflicts_#{crdt_type}")
    :ok
  end

  @doc """
  Updates data size for a namespace.
  """
  @spec update_data_size(String.t(), integer()) :: :ok
  def update_data_size(namespace, bytes) do
    set(:"data_size_bytes_#{namespace}", bytes)
    :ok
  end

  @doc """
  Records network bytes sent.
  """
  @spec record_bytes_sent(integer()) :: :ok
  def record_bytes_sent(bytes) do
    inc(:network_bytes_sent, bytes)
    :ok
  end

  @doc """
  Records network bytes received.
  """
  @spec record_bytes_received(integer()) :: :ok
  def record_bytes_received(bytes) do
    inc(:network_bytes_received, bytes)
    :ok
  end

  @doc """
  Records a membership change.
  """
  @spec record_membership_change(atom()) :: :ok
  def record_membership_change(event) do
    inc(:membership_changes_total)
    inc(:"membership_#{event}")
    :ok
  end

  # Server Callbacks

  @impl true
  def init(_opts) do
    state = %{
      counters: %{},
      gauges: %{},
      histograms: %{},
      start_time: System.monotonic_time(:second)
    }

    {:ok, state}
  end

  @impl true
  def handle_cast({:inc, name, amount}, state) do
    current = Map.get(state.counters, name, 0)
    counters = Map.put(state.counters, name, current + amount)
    {:noreply, %{state | counters: counters}}
  end

  @impl true
  def handle_cast({:set, name, value}, state) do
    gauges = Map.put(state.gauges, name, value)
    {:noreply, %{state | gauges: gauges}}
  end

  @impl true
  def handle_cast({:observe, name, value}, state) do
    current = Map.get(state.histograms, name, [])
    # Keep last 1000 observations
    updated = Enum.take([value | current], 1000)
    histograms = Map.put(state.histograms, name, updated)
    {:noreply, %{state | histograms: histograms}}
  end

  @impl true
  def handle_cast(:reset, state) do
    {:noreply, %{state | counters: %{}, gauges: %{}, histograms: %{}}}
  end

  @impl true
  def handle_call(:get_all, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_call({:get_counter, name}, _from, state) do
    {:reply, Map.get(state.counters, name, 0), state}
  end

  @impl true
  def handle_call({:get_gauge, name}, _from, state) do
    {:reply, Map.get(state.gauges, name), state}
  end
end
