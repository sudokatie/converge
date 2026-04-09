defmodule Converge.Monitoring.HealthTest do
  use ExUnit.Case, async: false

  alias Converge.Monitoring.Health

  describe "health/0" do
    test "returns :ok" do
      assert Health.health() == :ok
    end
  end

  describe "ready/0" do
    setup do
      # Start dependencies for ready check
      {:ok, store} = Converge.Storage.Store.start_link(data_dir: System.tmp_dir!())
      {:ok, node} = Converge.Cluster.Node.start_link([])
      {:ok, membership} = Converge.Sync.Membership.start_link(enabled: false)

      on_exit(fn ->
        Process.alive?(store) && GenServer.stop(store)
        Process.alive?(node) && GenServer.stop(node)
        Process.alive?(membership) && GenServer.stop(membership)
      end)

      :ok
    end

    test "returns :ok when all services are running" do
      assert Health.ready() == :ok
    end
  end

  describe "ready/0 with missing services" do
    test "returns error when storage is not running" do
      # No setup - services not started
      result = Health.ready()
      assert {:error, failed} = result
      assert is_list(failed)
    end
  end

  describe "HTTP server" do
    setup do
      # Start metrics for /metrics endpoint
      {:ok, metrics} = Converge.Monitoring.Metrics.start_link([])

      # Start health server on a random port
      port = Enum.random(49152..65535)
      {:ok, health} = Health.start_link(port: port, enabled: true)

      # Give server time to start
      Process.sleep(100)

      on_exit(fn ->
        Process.alive?(health) && GenServer.stop(health)
        Process.alive?(metrics) && GenServer.stop(metrics)
      end)

      {:ok, port: port}
    end

    test "responds to /health", %{port: port} do
      {:ok, conn} = :gen_tcp.connect(~c"127.0.0.1", port, [:binary, active: false])
      :gen_tcp.send(conn, "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
      {:ok, response} = :gen_tcp.recv(conn, 0, 1000)
      :gen_tcp.close(conn)

      assert String.contains?(response, "200 OK")
      assert String.contains?(response, "\"status\":\"ok\"")
    end

    test "responds to /metrics", %{port: port} do
      {:ok, conn} = :gen_tcp.connect(~c"127.0.0.1", port, [:binary, active: false])
      :gen_tcp.send(conn, "GET /metrics HTTP/1.1\r\nHost: localhost\r\n\r\n")
      {:ok, response} = :gen_tcp.recv(conn, 0, 1000)
      :gen_tcp.close(conn)

      assert String.contains?(response, "200 OK")
      assert String.contains?(response, "# Converge Metrics")
    end

    test "returns 404 for unknown paths", %{port: port} do
      {:ok, conn} = :gen_tcp.connect(~c"127.0.0.1", port, [:binary, active: false])
      :gen_tcp.send(conn, "GET /unknown HTTP/1.1\r\nHost: localhost\r\n\r\n")
      {:ok, response} = :gen_tcp.recv(conn, 0, 1000)
      :gen_tcp.close(conn)

      assert String.contains?(response, "404")
    end
  end
end
