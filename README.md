# Lattice

A CRDT database that actually works. Eventually consistent, conflict-free, and surprisingly pleasant to use.

## Why This Exists

Distributed systems are hard. Coordination is expensive. What if your data structures just... figured it out?

Lattice implements Conflict-free Replicated Data Types (CRDTs) - mathematical structures that merge automatically without coordination. Two nodes can make concurrent updates, sync whenever they feel like it, and always converge to the same state. No locks. No consensus protocols. Just math.

## Features

- **G-Counter** - Grow-only counter. Simple, fast, gets the job done.
- **PN-Counter** - Increment and decrement. Because sometimes you need to go backwards.
- **LWW-Register** - Last-writer-wins. Timestamps settle disputes.
- **OR-Set** - Add/remove elements. Add wins over concurrent remove (the sane choice).
- **LWW-Map** - Key-value with LWW semantics. Your distributed config store.

Plus:
- ETS/DETS storage (fast reads, durable writes)
- Merkle tree sync (efficient diffing)
- SWIM-based cluster membership (gossip that works)
- mDNS discovery (zero-config clustering)

## Quick Start

```elixir
# Counters
Lattice.counter_inc("myapp", "page_views")
Lattice.counter_value("myapp", "page_views")
# => 1

# Sets
Lattice.set_add("myapp", "tags", "elixir")
Lattice.set_add("myapp", "tags", "crdt")
Lattice.set_members("myapp", "tags")
# => ["elixir", "crdt"]

# Maps
Lattice.map_put("myapp", "user:1", "name", "Alice")
Lattice.map_get("myapp", "user:1", "name")
# => "Alice"

# Registers
Lattice.register_set("myapp", "config", %{theme: "dark"})
Lattice.register_get("myapp", "config")
# => %{theme: "dark"}
```

## CLI

```bash
# Build the CLI
mix escript.build

# Counter operations
./lattice counter inc myapp/visits
./lattice counter get myapp/visits

# Set operations
./lattice set add myapp/tags elixir
./lattice set members myapp/tags

# Cluster status
./lattice cluster status

# Trigger sync
./lattice sync
```

## How It Works

1. **Local-first writes** - Every operation hits local storage immediately. No network round-trips.

2. **Background sync** - Anti-entropy process periodically compares Merkle roots with peers. Different? Exchange the deltas.

3. **Automatic merge** - CRDTs have mathematically-defined merge functions. Concurrent updates? Merge handles it.

4. **Eventual consistency** - Given enough time and network connectivity, all nodes converge. Guaranteed.

## Configuration

```elixir
config :lattice,
  data_dir: "/var/lib/lattice",
  node_id: "node-1",
  sync_interval_ms: 5_000,
  enable_discovery: true
```

## Architecture

```
Lattice.Supervisor
  ├── Lattice.Cluster.Node       # Identity & peer tracking
  ├── Lattice.Storage.Store      # ETS/DETS backend
  ├── Lattice.Storage.WAL        # Write-ahead log
  ├── Lattice.Storage.Snapshot   # Periodic snapshots
  ├── Lattice.Sync.AntiEntropy   # Merkle-based sync
  ├── Lattice.Cluster.Discovery  # mDNS peer discovery
  └── Lattice.Cluster.Membership # SWIM protocol
```

## The Math (briefly)

CRDTs work because they're join-semilattices. Fancy term, simple idea:
- There's a partial order on states
- Any two states have a least upper bound (the merge)
- Merging is associative, commutative, and idempotent

Translation: merge order doesn't matter, re-merging is harmless, and everything converges.

## Limitations

- **Memory** - Everything lives in ETS. Huge datasets need more RAM.
- **Tombstones** - OR-Set keeps deleted element tags around. Clean them periodically.
- **Clock drift** - LWW types assume reasonable clock sync. NTP is your friend.
- **No transactions** - This is AP, not ACID. Design accordingly.

## License

MIT

---

*Built by Katie. Because distributed systems should be less painful.*
