defmodule Lattice.Query do
  @moduledoc """
  Lattice Query Language (LQL) - SQL-like queries for CRDT data.

  ## Quick Start

      # Create some data
      users = LWWMap.new("node1")
      users = LWWMap.put(users, "user1", %{name: "Alice", age: 30, active: true})
      users = LWWMap.put(users, "user2", %{name: "Bob", age: 25, active: false})

      # Query it
      data = %{"users" => users}

      {:ok, results} = Lattice.Query.run("SELECT * FROM users WHERE active = true", data)
      # => [%{name: "Alice", age: 30, active: true, _key: "user1"}]

      {:ok, count} = Lattice.Query.run("SELECT count(*) FROM users", data)
      # => 2

  ## Syntax

      SELECT [fields | * | count(*)] FROM collection [WHERE condition] [LIMIT n]

  ## Operators

  - Comparison: =, !=, <, >, <=, >=
  - Logical: AND, OR, NOT
  - Aggregates: count(*), keys(*), values(*)
  """

  alias Lattice.Query.{Parser, Executor}

  @doc """
  Parse and execute a query against a data source.

  ## Examples

      Lattice.Query.run("SELECT * FROM users", %{"users" => users_map})
      Lattice.Query.run("SELECT name FROM users WHERE age > 21", data)
      Lattice.Query.run("SELECT count(*) FROM events", data)
  """
  def run(query_string, data_source) when is_binary(query_string) do
    with {:ok, query} <- Parser.parse(query_string) do
      Executor.execute(query, data_source)
    end
  end

  @doc """
  Parse a query string without executing it.
  Useful for validation or query introspection.
  """
  def parse(query_string) do
    Parser.parse(query_string)
  end

  @doc """
  Execute a pre-parsed query against a data source.
  """
  def execute(query, data_source) do
    Executor.execute(query, data_source)
  end
end
