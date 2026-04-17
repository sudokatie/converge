# Lattice

A survival game engine built from scratch in Rust. Voxel world, building, creatures, multiplayer - the whole deal.

## Why This Exists

Because sometimes you want to understand how the sausage is made. No Unity. No Unreal. No Godot. Just raw Rust and too much ambition.

Lattice is a from-scratch game engine targeting the survival/voxel genre. Think Minecraft meets Valheim, but we actually understand every line of code. Whether that's a feature or a bug depends on the day.

## Current Status

**Milestones Complete: 7/7** - v0.1.0 Complete!

- [x] M1: Foundation - Workspace, math, platform, basic rendering
- [x] M2: Voxel World - Chunks, generation, meshing, persistence
- [x] M3: Player - Movement, collision, inventory
- [x] M4: Gameplay - Items, crafting, survival, creatures
- [x] M5: Multiplayer - Networking, sync, prediction, chat
- [x] M6: Content - Biomes, trees, hostile AI, combat
- [x] M7: Polish - Audio, menus, settings, profiling

## Architecture

```
lattice/
├── crates/
│   ├── engine_core/     # Math, memory, platform
│   ├── engine_render/   # wgpu graphics, voxel rendering
│   ├── engine_world/    # Chunk generation, persistence
│   ├── engine_physics/  # Collision, movement
│   ├── engine_network/  # Multiplayer networking
│   ├── engine_audio/    # Sound and music
│   ├── engine_ai/       # Creature behavior
│   ├── engine_ui/       # egui-based HUD and menus
│   ├── game/            # Game logic, ECS
│   └── server/          # Dedicated server
└── assets/
    └── data/            # RON config files
```

## Features

### World
- 16x16x16 chunks with greedy meshing
- Procedural terrain with caves
- Region-based persistence (lz4 compressed)
- Frustum culling and ambient occlusion
- Procedural sky with atmospheric scattering
- Day/night cycle with sun, moon, and stars

### Gameplay
- Data-driven items and recipes (RON files)
- Crafting with station requirements
- Health with damage/healing/invincibility
- Hunger with saturation buffer
- 36-slot inventory with hotbar

### Creatures
- Passive animals (pig, cow, sheep, chicken)
- Hostile mobs (zombie, skeleton, spider, creeper)
- Behavior tree AI with blackboard
- A* pathfinding on voxel grid

### Multiplayer
- Client-server architecture (renet)
- Server-authoritative with client prediction
- Input reconciliation and lag compensation
- Entity relevancy and snapshot system
- Chat with message history

### UI
- Health bar with damage flash
- Hunger bar with low-hunger shake
- Hotbar with item icons
- Crafting screen with filtering
- Main menu, pause menu, settings
- Debug overlay with FPS, position, memory

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test --workspace

# Run the game
cargo run
```

## Configuration

Items and recipes are data-driven via RON files in `assets/data/`:

```ron
// items.ron
(
    id: 100,
    name: "Wooden Pickaxe",
    stack_size: 1,
    category: Tool,
    tool_type: Some(Pickaxe),
    durability: Some(60),
)

// recipes.ron
(
    id: "wooden_pickaxe",
    inputs: [(item: "Oak Planks", count: 3), (item: "Stick", count: 2)],
    output: (item: "Wooden Pickaxe", count: 1),
    station: Some(CraftingTable),
)
```

## Dependencies

- **wgpu** - Cross-platform graphics
- **winit** - Window management
- **glam** - Math (vectors, matrices, quaternions)
- **hecs** - Entity Component System
- **egui** - Immediate mode UI
- **noise** - Terrain generation
- **serde/ron** - Configuration

## Performance Targets

- 60 FPS at 1080p
- 12 chunk view distance
- <2GB RAM
- 10 players multiplayer

## Philosophy

1. **Understand everything** - No magic black boxes
2. **Data-driven** - RON files over hardcoded values
3. **Test everything** - 540+ tests and counting
4. **One component, one file** - Clean architecture
5. **Build incrementally** - Task by task, milestone by milestone

## License

MIT

---

*Built by Katie. Because game engines are just really ambitious side projects.*
