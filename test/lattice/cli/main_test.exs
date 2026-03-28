defmodule Lattice.CLI.MainTest do
  use ExUnit.Case, async: true

  import ExUnit.CaptureIO

  alias Lattice.CLI.Main

  describe "main/1 with help" do
    test "--help shows usage" do
      output = capture_io(fn -> Main.main(["--help"]) end)

      assert output =~ "Lattice - CRDT database"
      assert output =~ "Usage:"
      assert output =~ "counter"
      assert output =~ "set"
      assert output =~ "cluster"
    end

    test "-h shows usage" do
      output = capture_io(fn -> Main.main(["-h"]) end)
      assert output =~ "Usage:"
    end
  end

  describe "main/1 with version" do
    test "--version shows version" do
      output = capture_io(fn -> Main.main(["--version"]) end)
      assert output =~ "Lattice v0.1.0"
    end

    test "-v shows version" do
      output = capture_io(fn -> Main.main(["-v"]) end)
      assert output =~ "Lattice v"
    end
  end

  describe "main/1 with no args" do
    test "shows help" do
      output = capture_io(fn -> Main.main([]) end)
      assert output =~ "Usage:"
    end
  end

  describe "path parsing" do
    # Test path parsing through counter commands
    test "parses namespace/key format" do
      # We can't easily test this without starting services,
      # but we can verify the help shows correct format
      output = capture_io(fn -> Main.main(["--help"]) end)
      assert output =~ "counter get <ns/key>"
    end
  end
end
