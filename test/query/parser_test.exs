defmodule Converge.Query.ParserTest do
  use ExUnit.Case, async: true

  alias Converge.Query.Parser

  describe "parse/1" do
    test "parses simple SELECT *" do
      {:ok, query} = Parser.parse("SELECT * FROM users")

      assert query.select == [:all]
      assert query.from == "users"
      assert query.where == nil
      assert query.limit == nil
    end

    test "parses SELECT with specific fields" do
      {:ok, query} = Parser.parse("SELECT name, email FROM users")

      assert query.select == ["name", "email"]
      assert query.from == "users"
    end

    test "parses SELECT with WHERE clause" do
      {:ok, query} = Parser.parse("SELECT * FROM users WHERE active = true")

      assert query.select == [:all]
      assert query.from == "users"
      assert query.where == {:eq, "active", true}
    end

    test "parses WHERE with number comparison" do
      {:ok, query} = Parser.parse("SELECT * FROM orders WHERE total > 100")

      assert query.where == {:gt, "total", 100}
    end

    test "parses WHERE with string comparison" do
      {:ok, query} = Parser.parse("SELECT * FROM users WHERE name = 'Alice'")

      assert query.where == {:eq, "name", "Alice"}
    end

    test "parses WHERE with AND" do
      {:ok, query} = Parser.parse("SELECT * FROM users WHERE active = true AND age > 18")

      assert query.where == {:and, {:eq, "active", true}, {:gt, "age", 18}}
    end

    test "parses WHERE with OR" do
      {:ok, query} = Parser.parse("SELECT * FROM users WHERE role = 'admin' OR role = 'mod'")

      assert query.where == {:or, {:eq, "role", "admin"}, {:eq, "role", "mod"}}
    end

    test "parses LIMIT clause" do
      {:ok, query} = Parser.parse("SELECT * FROM users LIMIT 10")

      assert query.limit == 10
    end

    test "parses count(*) function" do
      {:ok, query} = Parser.parse("SELECT count(*) FROM users")

      assert query.select == [{:function, :count, [:all]}]
    end

    test "parses keys(*) function" do
      {:ok, query} = Parser.parse("SELECT keys(*) FROM config")

      assert query.select == [{:function, :keys, [:all]}]
    end

    test "parses complex query" do
      {:ok, query} =
        Parser.parse(
          "SELECT name, email FROM users WHERE active = true AND age >= 21 LIMIT 50"
        )

      assert query.select == ["name", "email"]
      assert query.from == "users"
      assert query.where == {:and, {:eq, "active", true}, {:gte, "age", 21}}
      assert query.limit == 50
    end
  end

  describe "tokenize/1" do
    test "tokenizes simple query" do
      tokens = Parser.tokenize("SELECT * FROM users")
      assert tokens == [:select, :all, :from, "users"]
    end

    test "tokenizes operators" do
      tokens = Parser.tokenize("a = 1 AND b > 2")
      assert tokens == ["a", :eq, 1, :and, "b", :gt, 2]
    end

    test "tokenizes quoted strings" do
      tokens = Parser.tokenize("name = 'John Doe'")
      assert tokens == ["name", :eq, "John Doe"]
    end
  end
end
