defmodule Lattice.Sync.ProtocolTest do
  use ExUnit.Case, async: true

  alias Lattice.Sync.Protocol

  describe "sync_request/3" do
    test "creates valid sync request" do
      root = :crypto.hash(:sha256, "test")
      msg = Protocol.sync_request("default", root, "node-1")

      assert msg.type == :sync_request
      assert msg.namespace == "default"
      assert msg.merkle_root == root
      assert msg.from_node == "node-1"
    end
  end

  describe "sync_response/3" do
    test "creates valid sync response" do
      entries = [{"key1", %{value: 1}}, {"key2", %{value: 2}}]
      msg = Protocol.sync_response("default", entries, "node-1")

      assert msg.type == :sync_response
      assert msg.namespace == "default"
      assert msg.entries == entries
      assert msg.from_node == "node-1"
    end

    test "handles empty entries" do
      msg = Protocol.sync_response("ns", [], "node-1")
      assert msg.entries == []
    end
  end

  describe "update/4" do
    test "creates valid update" do
      crdt = %{type: :g_counter, value: 5}
      msg = Protocol.update("default", "mykey", crdt, "node-1")

      assert msg.type == :update
      assert msg.namespace == "default"
      assert msg.key == "mykey"
      assert msg.crdt_state == crdt
      assert msg.from_node == "node-1"
    end
  end

  describe "ping/1 and pong/1" do
    test "creates ping with timestamp" do
      msg = Protocol.ping("node-1")

      assert msg.type == :ping
      assert msg.from_node == "node-1"
      assert is_integer(msg.timestamp)
    end

    test "creates pong with timestamp" do
      msg = Protocol.pong("node-2")

      assert msg.type == :pong
      assert msg.from_node == "node-2"
      assert is_integer(msg.timestamp)
    end
  end

  describe "encode/1 and decode/1" do
    test "roundtrip sync_request" do
      root = :crypto.hash(:sha256, "test")
      msg = Protocol.sync_request("ns", root, "node-1")

      encoded = Protocol.encode(msg)
      assert is_binary(encoded)

      {:ok, decoded} = Protocol.decode(encoded)
      assert decoded == msg
    end

    test "roundtrip sync_response" do
      entries = [{"k1", %{v: 1}}]
      msg = Protocol.sync_response("ns", entries, "node-1")

      encoded = Protocol.encode(msg)
      {:ok, decoded} = Protocol.decode(encoded)
      assert decoded == msg
    end

    test "roundtrip update" do
      msg = Protocol.update("ns", "key", %{state: true}, "node-1")

      encoded = Protocol.encode(msg)
      {:ok, decoded} = Protocol.decode(encoded)
      assert decoded == msg
    end

    test "decode invalid binary returns error" do
      {:error, :invalid_binary} = Protocol.decode(<<0, 1, 2, 3>>)
    end

    test "decode invalid message type returns error" do
      invalid = :erlang.term_to_binary(%{type: :unknown_type, data: 123})
      {:error, :unknown_message_type} = Protocol.decode(invalid)
    end
  end

  describe "message_type/1" do
    test "returns type for each message kind" do
      assert Protocol.message_type(Protocol.sync_request("ns", <<>>, "n")) == :sync_request
      assert Protocol.message_type(Protocol.sync_response("ns", [], "n")) == :sync_response
      assert Protocol.message_type(Protocol.update("ns", "k", %{}, "n")) == :update
      assert Protocol.message_type(Protocol.ping("n")) == :ping
      assert Protocol.message_type(Protocol.pong("n")) == :pong
    end
  end

  describe "handle_message/2" do
    defmodule TestHandler do
      def handle_sync_request(msg), do: {:ok, [{:received, msg.type}]}
      def handle_sync_response(msg), do: {:ok, [{:received, msg.type}]}
      def handle_update(msg), do: {:ok, [{:received, msg.type}]}
      def handle_ping(_msg), do: {:ok, [Protocol.pong("test-node")]}
      def handle_pong(_msg), do: {:ok, []}
    end

    test "dispatches sync_request" do
      msg = Protocol.sync_request("ns", <<>>, "node-1")
      {:ok, [{:received, :sync_request}]} = Protocol.handle_message(msg, TestHandler)
    end

    test "dispatches sync_response" do
      msg = Protocol.sync_response("ns", [], "node-1")
      {:ok, [{:received, :sync_response}]} = Protocol.handle_message(msg, TestHandler)
    end

    test "dispatches update" do
      msg = Protocol.update("ns", "key", %{}, "node-1")
      {:ok, [{:received, :update}]} = Protocol.handle_message(msg, TestHandler)
    end

    test "dispatches ping and returns pong" do
      msg = Protocol.ping("node-1")
      {:ok, [response]} = Protocol.handle_message(msg, TestHandler)
      assert response.type == :pong
    end

    test "dispatches pong" do
      msg = Protocol.pong("node-1")
      {:ok, []} = Protocol.handle_message(msg, TestHandler)
    end

    test "unknown message type returns error" do
      {:error, :unknown_message_type} = Protocol.handle_message(%{type: :bad}, TestHandler)
    end
  end
end
