defmodule Lattice.Application do
  @moduledoc """
  OTP Application for Lattice CRDT database.
  """
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      # Will add supervision tree as we build modules:
      # Lattice.Config,
      # Lattice.Cluster.Node,
      # Lattice.Storage.Store,
      # Lattice.Storage.WAL,
      # Lattice.Storage.Snapshot,
      # Lattice.Sync.AntiEntropy,
      # Lattice.Cluster.Discovery,
      # Lattice.Cluster.Membership
    ]

    opts = [strategy: :one_for_one, name: Lattice.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
