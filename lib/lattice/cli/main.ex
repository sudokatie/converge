defmodule Lattice.CLI.Main do
  @moduledoc """
  Main CLI entry point for Lattice.
  """

  alias Lattice.CLI.Commands

  @doc """
  Main entry point for escript.
  """
  def main(args) do
    {opts, args, _invalid} =
      OptionParser.parse(args,
        strict: [
          help: :boolean,
          version: :boolean,
          data_dir: :string,
          node_id: :string
        ],
        aliases: [
          h: :help,
          v: :version,
          d: :data_dir,
          n: :node_id
        ]
      )

    cond do
      opts[:help] ->
        print_help()

      opts[:version] ->
        print_version()

      true ->
        run_command(args, opts)
    end
  end

  defp run_command([], _opts) do
    print_help()
  end

  defp run_command(["start" | _rest], opts) do
    Commands.start(opts)
  end

  defp run_command(["cluster", "status" | _rest], _opts) do
    Commands.cluster_status()
  end

  defp run_command(["cluster", "join", seed | _rest], _opts) do
    Commands.cluster_join(seed)
  end

  defp run_command(["cluster", "leave" | _rest], _opts) do
    Commands.cluster_leave()
  end

  defp run_command(["counter", "get", path | _rest], _opts) do
    {ns, key} = parse_path(path)
    Commands.counter_get(ns, key)
  end

  defp run_command(["counter", "inc", path | rest], _opts) do
    {ns, key} = parse_path(path)
    amount = parse_amount(rest)
    Commands.counter_inc(ns, key, amount)
  end

  defp run_command(["counter", "dec", path | rest], _opts) do
    {ns, key} = parse_path(path)
    amount = parse_amount(rest)
    Commands.counter_dec(ns, key, amount)
  end

  defp run_command(["set", "members", path | _rest], _opts) do
    {ns, key} = parse_path(path)
    Commands.set_members(ns, key)
  end

  defp run_command(["set", "add", path, element | _rest], _opts) do
    {ns, key} = parse_path(path)
    Commands.set_add(ns, key, element)
  end

  defp run_command(["set", "remove", path, element | _rest], _opts) do
    {ns, key} = parse_path(path)
    Commands.set_remove(ns, key, element)
  end

  defp run_command(["namespace", "list" | _rest], _opts) do
    Commands.namespace_list()
  end

  defp run_command(["namespace", "create", name | _rest], _opts) do
    Commands.namespace_create(name)
  end

  defp run_command(["sync" | rest], _opts) do
    namespace = List.first(rest)
    Commands.sync(namespace)
  end

  defp run_command([cmd | _rest], _opts) do
    IO.puts("Unknown command: #{cmd}")
    IO.puts("Run 'lattice --help' for usage.")
    System.halt(1)
  end

  defp parse_path(path) do
    case String.split(path, "/", parts: 2) do
      [ns, key] -> {ns, key}
      [key] -> {"default", key}
    end
  end

  defp parse_amount([]), do: 1

  defp parse_amount([amount_str | _]) do
    case Integer.parse(amount_str) do
      {amount, _} -> amount
      :error -> 1
    end
  end

  defp print_help do
    IO.puts("""
    Lattice - CRDT database with automatic conflict resolution

    Usage: lattice [options] <command> [args]

    Options:
      -h, --help        Show this help
      -v, --version     Show version
      -d, --data-dir    Data directory path
      -n, --node-id     Node identifier

    Commands:
      start                     Start the Lattice node
      cluster status            Show cluster status
      cluster join <seed>       Join cluster via seed node
      cluster leave             Leave the cluster gracefully

      counter get <ns/key>      Get counter value
      counter inc <ns/key> [n]  Increment counter
      counter dec <ns/key> [n]  Decrement counter

      set members <ns/key>      List set members
      set add <ns/key> <elem>   Add element to set
      set remove <ns/key> <e>   Remove element from set

      namespace list            List all namespaces
      namespace create <name>   Create namespace

      sync [namespace]          Trigger sync

    Examples:
      lattice counter inc myapp/visits
      lattice set add myapp/tags elixir
      lattice cluster status
    """)
  end

  defp print_version do
    IO.puts("Lattice v0.1.0")
  end
end
