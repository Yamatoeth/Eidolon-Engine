# emergent-sim — 3D ECS Simulation Engine in Rust

> A real-time 3D simulation engine built in Rust with Bevy, demonstrating emergent behavior, utility-based AI, and advanced observability tooling.

[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/bevy-0.14-blue)](https://bevyengine.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## What is this?

**emergent-sim** is not a game. It is a technical demonstration of a real-time 3D simulation engine built from the ground up in Rust, using the [Bevy](https://bevyengine.org/) ECS framework.

The project showcases:
- Clean ECS architecture with strict separation of concerns
- Utility-based AI producing emergent, script-free agent behavior
- Fully deterministic simulation with seed-based reproducibility
- Advanced debug tooling: entity inspector, event timeline, replay system
- Data-driven design and modular system composition

The "world" is a compact 3D environment where autonomous agents navigate, compete for resources, and produce complex collective dynamics from simple local rules.

---

## Key Features

| Feature | Description |
|---|---|
| **ECS Architecture** | Clean component/system separation via Bevy |
| **Utility AI** | Agents score and rank actions dynamically |
| **Emergent Behavior** | Complex dynamics from simple rules |
| **Deterministic Sim** | Seed-based, fully reproducible |
| **Entity Inspector** | Live ECS state visualization |
| **Event Timeline** | Causal chain visualization |
| **Replay System** | Rewind/fast-forward any simulation |
| **Debug Overlays** | AI decision visualization in 3D |
| **Scenario System** | Preconfigured simulation presets |

---

## Quick Start

```bash
git clone https://github.com/yourname/emergent-sim
cd emergent-sim
cargo run --release
```

Controls:
- `WASD` + Mouse — orbit camera
- `Left Click` — inspect entity
- `F1` — toggle Entity Inspector
- `F2` — toggle Event Timeline
- `F3` — toggle Debug Overlays
- `Space` — pause/resume simulation
- `R` — restart with same seed
- `N` — new seed

---

## Documentation Index

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | System layers, module boundaries, data flow |
| [ECS_DESIGN.md](docs/ECS_DESIGN.md) | Components, systems, resources, events |
| [AI_SYSTEM.md](docs/AI_SYSTEM.md) | Utility scoring, agent decision pipeline |
| [SIMULATION.md](docs/SIMULATION.md) | World rules, spatial system, resource dynamics |
| [OBSERVABILITY.md](docs/OBSERVABILITY.md) | Inspector, timeline, replay, overlays |
| [SCENARIOS.md](docs/SCENARIOS.md) | Predefined simulation configurations |
| [ROADMAP.md](docs/ROADMAP.md) | Development phases and milestones |
| [CONTRIBUTING.md](docs/CONTRIBUTING.md) | Code style, conventions, PR process |

---

## Project Structure

```
emergent-sim/
├── src/
│   ├── main.rs
│   ├── engine/          # Rendering, input, camera, ECS loop
│   ├── simulation/      # World logic, agents, resources, events
│   ├── ai/              # Utility scoring, decision systems
│   ├── observability/   # Inspector, timeline, replay, overlays
│   └── scenarios/       # Preconfigured world setups
├── assets/
│   ├── meshes/
│   ├── materials/
│   └── scenarios/       # JSON scenario definitions
├── docs/
│   └── ...              # All documentation files
└── tests/
    ├── simulation/
    └── determinism/
```

---

## Why Rust + Bevy?

- **Rust** enforces correctness at compile time — no data races, no null, no unsafe memory in simulation-critical code
- **Bevy ECS** provides cache-friendly, parallel-friendly data layout with a clean systems API
- The combination makes complex simulation state manageable and verifiable

---

## Portfolio Context

This project is built as a technical portfolio piece targeting **game engine**, **game systems**, and **real-time backend** engineering roles. It is intentionally scoped to demonstrate engineering depth over breadth — the goal is a well-architected, well-instrumented, and reproducible simulation core, not a playable product.

See [ROADMAP.md](docs/ROADMAP.md) for the phased development plan.
