defmodule Lattice.Sync.Protocol do
  @moduledoc """
  Sync protocol message types and handling.

  Defines message structs for node-to-node communication:
  - SyncRequest: request sync state for namespace
  - SyncResponse: response with missing keys/CRDTs
  - Update: direct update push
  """

  # Message type atoms
  @type_sync_request :sync_request
  @type_sync_response :sync_response
  @type_update :update
  @type_ping :ping
  @type_pong :pong

  @type namespace :: String.t()
  @type key :: String.t()
  @type node_id :: String.t()
  @type crdt_state :: term()
  @type merkle_root :: binary()

  @type message ::
          sync_request()
          | sync_response()
          | update()
          | ping()
          | pong()

  @type sync_request :: %{
          type: :sync_request,
          namespace: namespace(),
          merkle_root: merkle_root(),
          from_node: node_id()
        }

  @type sync_response :: %{
          type: :sync_response,
          namespace: namespace(),
          entries: [{key(), crdt_state()}],
          from_node: node_id()
        }

  @type update :: %{
          type: :update,
          namespace: namespace(),
          key: key(),
          crdt_state: crdt_state(),
          from_node: node_id()
        }

  @type ping :: %{type: :ping, from_node: node_id(), timestamp: integer()}
  @type pong :: %{type: :pong, from_node: node_id(), timestamp: integer()}

  @doc """
  Creates a sync request message.
  """
  @spec sync_request(namespace(), merkle_root(), node_id()) :: sync_request()
  def sync_request(namespace, merkle_root, from_node) do
    %{
      type: @type_sync_request,
      namespace: namespace,
      merkle_root: merkle_root,
      from_node: from_node
    }
  end

  @doc """
  Creates a sync response message.
  """
  @spec sync_response(namespace(), [{key(), crdt_state()}], node_id()) :: sync_response()
  def sync_response(namespace, entries, from_node) do
    %{
      type: @type_sync_response,
      namespace: namespace,
      entries: entries,
      from_node: from_node
    }
  end

  @doc """
  Creates an update message.
  """
  @spec update(namespace(), key(), crdt_state(), node_id()) :: update()
  def update(namespace, key, crdt_state, from_node) do
    %{
      type: @type_update,
      namespace: namespace,
      key: key,
      crdt_state: crdt_state,
      from_node: from_node
    }
  end

  @doc """
  Creates a ping message.
  """
  @spec ping(node_id()) :: ping()
  def ping(from_node) do
    %{
      type: @type_ping,
      from_node: from_node,
      timestamp: System.monotonic_time(:millisecond)
    }
  end

  @doc """
  Creates a pong message.
  """
  @spec pong(node_id()) :: pong()
  def pong(from_node) do
    %{
      type: @type_pong,
      from_node: from_node,
      timestamp: System.monotonic_time(:millisecond)
    }
  end

  @doc """
  Encodes a message for transmission.
  """
  @spec encode(message()) :: binary()
  def encode(message) do
    :erlang.term_to_binary(message)
  end

  @doc """
  Decodes a message from binary.

  Returns {:ok, message} or {:error, reason}.
  """
  @spec decode(binary()) :: {:ok, message()} | {:error, term()}
  def decode(binary) do
    try do
      term = :erlang.binary_to_term(binary, [:safe])
      validate_message(term)
    rescue
      ArgumentError -> {:error, :invalid_binary}
    end
  end

  @doc """
  Returns the message type.
  """
  @spec message_type(message()) :: atom()
  def message_type(%{type: type}), do: type

  @doc """
  Handles an incoming message, dispatching to the appropriate handler.

  Returns {:ok, response_messages} or {:error, reason}.
  The handler_module must implement handle_sync_request/1, handle_sync_response/1, etc.
  """
  @spec handle_message(message(), module()) :: {:ok, [message()]} | {:error, term()}
  def handle_message(%{type: @type_sync_request} = msg, handler) do
    handler.handle_sync_request(msg)
  end

  def handle_message(%{type: @type_sync_response} = msg, handler) do
    handler.handle_sync_response(msg)
  end

  def handle_message(%{type: @type_update} = msg, handler) do
    handler.handle_update(msg)
  end

  def handle_message(%{type: @type_ping} = msg, handler) do
    handler.handle_ping(msg)
  end

  def handle_message(%{type: @type_pong} = msg, handler) do
    handler.handle_pong(msg)
  end

  def handle_message(_unknown, _handler) do
    {:error, :unknown_message_type}
  end

  # Private functions

  defp validate_message(%{type: @type_sync_request} = msg) do
    if is_binary(msg[:namespace]) and is_binary(msg[:merkle_root]) and is_binary(msg[:from_node]) do
      {:ok, msg}
    else
      {:error, :invalid_sync_request}
    end
  end

  defp validate_message(%{type: @type_sync_response} = msg) do
    if is_binary(msg[:namespace]) and is_list(msg[:entries]) and is_binary(msg[:from_node]) do
      {:ok, msg}
    else
      {:error, :invalid_sync_response}
    end
  end

  defp validate_message(%{type: @type_update} = msg) do
    if is_binary(msg[:namespace]) and is_binary(msg[:key]) and is_binary(msg[:from_node]) do
      {:ok, msg}
    else
      {:error, :invalid_update}
    end
  end

  defp validate_message(%{type: @type_ping} = msg) do
    if is_binary(msg[:from_node]) do
      {:ok, msg}
    else
      {:error, :invalid_ping}
    end
  end

  defp validate_message(%{type: @type_pong} = msg) do
    if is_binary(msg[:from_node]) do
      {:ok, msg}
    else
      {:error, :invalid_pong}
    end
  end

  defp validate_message(_) do
    {:error, :unknown_message_type}
  end
end
