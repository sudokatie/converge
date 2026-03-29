defmodule Lattice.Storage.WAL do
  @moduledoc """
  Write-Ahead Log for durability.

  Operations are written to the log before being applied.
  On recovery, the log is replayed to restore state.
  """
  use GenServer

  @type operation :: {:put, String.t(), any(), any()} | {:delete, String.t(), any()}

  defstruct [:path, :file, :checkpoint_pos]

  # Client API

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Append an operation to the log.
  """
  def append(operation) do
    GenServer.call(__MODULE__, {:append, operation})
  end

  @doc """
  Replay log entries since last checkpoint.
  """
  def replay do
    GenServer.call(__MODULE__, :replay)
  end

  @doc """
  Create a checkpoint (mark current position).
  """
  def checkpoint do
    GenServer.call(__MODULE__, :checkpoint)
  end

  @doc """
  Truncate log up to the checkpoint.
  """
  def truncate do
    GenServer.call(__MODULE__, :truncate)
  end

  @doc """
  Close the WAL file.
  """
  def close do
    GenServer.call(__MODULE__, :close)
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    data_dir = Keyword.get(opts, :data_dir, Lattice.Config.data_dir())
    File.mkdir_p!(data_dir)

    path = Path.join(data_dir, "wal.log")
    checkpoint_path = Path.join(data_dir, "wal.checkpoint")

    # Read checkpoint position
    checkpoint_pos =
      case File.read(checkpoint_path) do
        {:ok, content} -> String.to_integer(String.trim(content))
        {:error, _} -> 0
      end

    # Open file for append
    {:ok, file} = File.open(path, [:append, :binary])

    state = %__MODULE__{
      path: path,
      file: file,
      checkpoint_pos: checkpoint_pos
    }

    {:ok, state}
  end

  @impl true
  def handle_call({:append, operation}, _from, state) do
    entry = :erlang.term_to_binary(operation)
    size = byte_size(entry)

    # Write: [size:32-bit][entry]
    :ok = IO.binwrite(state.file, <<size::32>> <> entry)

    {:reply, :ok, state}
  end

  @impl true
  def handle_call(:replay, _from, state) do
    entries = read_entries(state.path, state.checkpoint_pos)
    {:reply, entries, state}
  end

  @impl true
  def handle_call(:checkpoint, _from, state) do
    # Get current file position
    {:ok, pos} = :file.position(state.file, :cur)

    # Write checkpoint
    checkpoint_path = Path.rootname(state.path) <> ".checkpoint"
    File.write!(checkpoint_path, Integer.to_string(pos))

    {:reply, :ok, %{state | checkpoint_pos: pos}}
  end

  @impl true
  def handle_call(:truncate, _from, state) do
    # Close current file
    File.close(state.file)

    # Read entries after checkpoint
    entries = read_entries(state.path, state.checkpoint_pos)

    # Rewrite file with only entries after checkpoint
    {:ok, file} = File.open(state.path, [:write, :binary])

    Enum.each(entries, fn entry ->
      binary = :erlang.term_to_binary(entry)
      size = byte_size(binary)
      IO.binwrite(file, <<size::32>> <> binary)
    end)

    # Update checkpoint to 0
    checkpoint_path = Path.rootname(state.path) <> ".checkpoint"
    File.write!(checkpoint_path, "0")

    {:reply, :ok, %{state | file: file, checkpoint_pos: 0}}
  end

  @impl true
  def handle_call(:close, _from, state) do
    File.close(state.file)
    {:reply, :ok, state}
  end

  # Private

  defp read_entries(path, from_pos) do
    case File.read(path) do
      {:ok, content} ->
        parse_entries(content, 0, from_pos, [])

      {:error, :enoent} ->
        []
    end
  end

  defp parse_entries(<<>>, _current_pos, _from_pos, acc), do: Enum.reverse(acc)

  defp parse_entries(<<size::32, rest::binary>>, current_pos, from_pos, acc)
       when byte_size(rest) >= size do
    <<entry::binary-size(size), remaining::binary>> = rest
    next_pos = current_pos + 4 + size

    if current_pos >= from_pos do
      operation = :erlang.binary_to_term(entry)
      parse_entries(remaining, next_pos, from_pos, [operation | acc])
    else
      parse_entries(remaining, next_pos, from_pos, acc)
    end
  end

  defp parse_entries(_, _current_pos, _from_pos, acc), do: Enum.reverse(acc)
end
