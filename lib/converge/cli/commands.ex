defmodule Converge.CLI.Commands do
  @moduledoc """
  CLI command implementations.
  """

  @doc """
  Starts the Converge application.
  """
  def start(opts) do
    data_dir = Keyword.get(opts, :data_dir, "/var/lib/converge")
    node_id = Keyword.get(opts, :node_id)

    IO.puts("Starting Converge node...")

    # Set application config
    Application.put_env(:converge, :data_dir, data_dir)

    if node_id do
      Application.put_env(:converge, :node_id, node_id)
    end

    # Start the application
    case Application.ensure_all_started(:converge) do
      {:ok, _} ->
        IO.puts("Converge started successfully.")
        IO.puts("Data directory: #{data_dir}")
        IO.puts("Press Ctrl+C to stop.")

        # Keep running
        Process.sleep(:infinity)

      {:error, reason} ->
        IO.puts("Failed to start Converge: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Shows cluster status.
  """
  def cluster_status do
    ensure_started()

    status = Converge.cluster_status()

    IO.puts("Cluster Status")
    IO.puts("==============")
    IO.puts("Node ID: #{status.node.id}")
    IO.puts("Members: #{status.member_count}")
    IO.puts("Namespaces: #{status.namespace_count}")

    if status.member_count > 0 do
      IO.puts("\nMembers:")

      Enum.each(status.members, fn member ->
        IO.puts("  - #{member.id} (#{member.address}:#{member.port})")
      end)
    end

    if status.namespace_count > 0 do
      IO.puts("\nNamespaces:")

      Enum.each(status.namespaces, fn ns ->
        IO.puts("  - #{ns}")
      end)
    end
  end

  @doc """
  Joins cluster via seed node.
  """
  def cluster_join(seed) do
    ensure_started()

    [address, port_str] =
      case String.split(seed, ":") do
        [addr, port] -> [addr, port]
        [addr] -> [addr, "4000"]
      end

    {port, _} = Integer.parse(port_str)

    seed_node = %{
      id: "seed-#{:rand.uniform(10000)}",
      address: address,
      port: port
    }

    case Converge.Sync.Membership.join(seed_node) do
      :ok ->
        IO.puts("Joined cluster via #{seed}")

      {:error, reason} ->
        IO.puts("Failed to join: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Leaves the cluster gracefully.
  """
  def cluster_leave do
    ensure_started()

    case Converge.Sync.Membership.leave() do
      :ok ->
        IO.puts("Left cluster successfully")

      {:error, reason} ->
        IO.puts("Failed to leave: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Gets a counter value.
  """
  def counter_get(namespace, key) do
    ensure_started()

    value = Converge.counter_value(namespace, key)
    IO.puts("#{namespace}/#{key}: #{value}")
  end

  @doc """
  Increments a counter.
  """
  def counter_inc(namespace, key, amount) do
    ensure_started()

    case Converge.counter_inc(namespace, key, amount) do
      :ok ->
        value = Converge.counter_value(namespace, key)
        IO.puts("#{namespace}/#{key}: #{value}")

      {:error, reason} ->
        IO.puts("Error: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Decrements a counter.
  """
  def counter_dec(namespace, key, amount) do
    ensure_started()

    case Converge.counter_dec(namespace, key, amount) do
      :ok ->
        value = Converge.counter_value(namespace, key)
        IO.puts("#{namespace}/#{key}: #{value}")

      {:error, reason} ->
        IO.puts("Error: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Lists set members.
  """
  def set_members(namespace, key) do
    ensure_started()

    members = Converge.set_members(namespace, key)

    if length(members) == 0 do
      IO.puts("(empty set)")
    else
      Enum.each(members, fn member ->
        IO.puts("  - #{inspect(member)}")
      end)
    end
  end

  @doc """
  Adds element to set.
  """
  def set_add(namespace, key, element) do
    ensure_started()

    case Converge.set_add(namespace, key, element) do
      :ok ->
        IO.puts("Added '#{element}' to #{namespace}/#{key}")

      {:error, reason} ->
        IO.puts("Error: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Removes element from set.
  """
  def set_remove(namespace, key, element) do
    ensure_started()

    case Converge.set_remove(namespace, key, element) do
      :ok ->
        IO.puts("Removed '#{element}' from #{namespace}/#{key}")

      {:error, reason} ->
        IO.puts("Error: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Lists all namespaces.
  """
  def namespace_list do
    ensure_started()

    case Converge.list_namespaces() do
      namespaces when is_list(namespaces) ->
        if length(namespaces) == 0 do
          IO.puts("(no namespaces)")
        else
          Enum.each(namespaces, fn ns ->
            IO.puts("  - #{ns}")
          end)
        end

      {:error, reason} ->
        IO.puts("Error: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Creates a namespace.
  """
  def namespace_create(name) do
    ensure_started()

    case Converge.create_namespace(name) do
      :ok ->
        IO.puts("Created namespace '#{name}'")

      {:error, reason} ->
        IO.puts("Error: #{inspect(reason)}")
        System.halt(1)
    end
  end

  @doc """
  Triggers sync.
  """
  def sync(namespace) do
    ensure_started()

    if namespace do
      Converge.sync_now(namespace)
      IO.puts("Sync triggered for namespace '#{namespace}'")
    else
      Converge.sync_now()
      IO.puts("Sync triggered for all namespaces")
    end
  end

  # Private

  defp ensure_started do
    unless Application.started_applications()
           |> Enum.any?(fn {app, _, _} -> app == :converge end) do
      # Start minimal services for CLI
      {:ok, _} = Converge.Storage.Store.start_link([])
      {:ok, _} = Converge.Cluster.Node.start_link([])
    end
  end
end
