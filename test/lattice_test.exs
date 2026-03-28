defmodule LatticeTest do
  use ExUnit.Case

  test "API stubs return not_implemented" do
    assert Lattice.counter_inc("ns", "key") == {:error, :not_implemented}
    assert Lattice.set_add("ns", "key", "elem") == {:error, :not_implemented}
    assert Lattice.map_get("ns", "key", "field") == {:error, :not_implemented}
  end

  test "module has documentation" do
    {:docs_v1, _, :elixir, _, %{"en" => doc}, _, _} = Code.fetch_docs(Lattice)
    assert String.contains?(doc, "CRDT")
  end
end
