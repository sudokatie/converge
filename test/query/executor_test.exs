defmodule Converge.Query.ExecutorTest do
  use ExUnit.Case, async: true

  alias Converge.Query
  alias Converge.CRDT.LWWMap

  setup do
    # Create test data
    users =
      LWWMap.new("node1")
      |> LWWMap.put("user1", %{"name" => "Alice", "age" => 30, "active" => true})
      |> LWWMap.put("user2", %{"name" => "Bob", "age" => 25, "active" => false})
      |> LWWMap.put("user3", %{"name" => "Charlie", "age" => 35, "active" => true})

    orders =
      LWWMap.new("node1")
      |> LWWMap.put("order1", %{"total" => 150, "status" => "shipped"})
      |> LWWMap.put("order2", %{"total" => 75, "status" => "pending"})
      |> LWWMap.put("order3", %{"total" => 200, "status" => "shipped"})

    data = %{
      "users" => users,
      "orders" => orders
    }

    {:ok, data: data}
  end

  describe "run/2" do
    test "selects all from collection", %{data: data} do
      {:ok, results} = Query.run("SELECT * FROM users", data)

      assert length(results) == 3
      assert Enum.all?(results, &Map.has_key?(&1, "_key"))
    end

    test "filters with WHERE clause", %{data: data} do
      {:ok, results} = Query.run("SELECT * FROM users WHERE active = true", data)

      assert length(results) == 2
      assert Enum.all?(results, &(&1["active"] == true))
    end

    test "filters with numeric comparison", %{data: data} do
      {:ok, results} = Query.run("SELECT * FROM users WHERE age > 28", data)

      assert length(results) == 2
      names = Enum.map(results, & &1["name"])
      assert "Alice" in names
      assert "Charlie" in names
    end

    test "filters with string comparison", %{data: data} do
      {:ok, results} = Query.run("SELECT * FROM users WHERE name = 'Bob'", data)

      assert length(results) == 1
      assert hd(results)["name"] == "Bob"
    end

    test "filters with AND condition", %{data: data} do
      {:ok, results} =
        Query.run("SELECT * FROM users WHERE active = true AND age > 30", data)

      assert length(results) == 1
      assert hd(results)["name"] == "Charlie"
    end

    test "filters with OR condition", %{data: data} do
      {:ok, results} =
        Query.run("SELECT * FROM orders WHERE status = 'shipped' OR total > 100", data)

      # order1: shipped, total=150
      # order2: pending, total=75 (excluded)
      # order3: shipped, total=200
      assert length(results) == 2
    end

    test "projects specific fields", %{data: data} do
      {:ok, results} = Query.run("SELECT name, age FROM users", data)

      assert length(results) == 3

      Enum.each(results, fn item ->
        assert Map.has_key?(item, "name")
        assert Map.has_key?(item, "age")
        refute Map.has_key?(item, "active")
      end)
    end

    test "applies LIMIT", %{data: data} do
      {:ok, results} = Query.run("SELECT * FROM users LIMIT 2", data)

      assert length(results) == 2
    end

    test "counts items", %{data: data} do
      {:ok, count} = Query.run("SELECT count(*) FROM users", data)

      assert count == 3
    end

    test "counts filtered items", %{data: data} do
      {:ok, count} = Query.run("SELECT count(*) FROM users WHERE active = true", data)

      assert count == 2
    end

    test "returns keys", %{data: data} do
      {:ok, keys} = Query.run("SELECT keys(*) FROM users", data)

      assert is_list(keys)
      # Should have keys like name, age, active, _key
    end

    test "handles collection not found" do
      {:error, msg} = Query.run("SELECT * FROM nonexistent", %{})

      assert msg =~ "not found"
    end
  end

  describe "complex queries" do
    test "combined filter and projection with limit", %{data: data} do
      {:ok, results} =
        Query.run(
          "SELECT name FROM users WHERE active = true LIMIT 1",
          data
        )

      assert length(results) == 1
      assert Map.has_key?(hd(results), "name")
    end
  end
end
