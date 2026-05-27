# emergent-sim — 3D ECS Simulation Engine in Rust

> A real-time 3D simulation engine built in Rust with Bevy 0.15, demonstrating emergent behavior, utility-based AI, and advanced observability tooling.

[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/bevy-0.15-blue)](https://bevyengine.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## What is this?

**emergent-sim** is not a game. It is a technical demonstration of a real-time 3D simulation engine built from the ground up in Rust, using the [Bevy 0.15](https://bevyengine.org/) ECS framework.

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
| **ECS Architecture** | Clean component/system separation via Bevy 0.15 |
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
| [.agent/ARCHITECTURE.md](.agent/ARCHITECTURE.md) | System layers, module boundaries, data flow |
| [.agent/ECS_DESIGN.md](.agent/ECS_DESIGN.md) | Components, systems, resources, events |
| [.agent/AI_SYSTEM.md](.agent/AI_SYSTEM.md) | Utility scoring, agent decision pipeline |
| [.agent/SIMULATION.md](.agent/SIMULATION.md) | World rules, spatial system, resource dynamics |
| [.agent/OBSERVABILITY.md](.agent/OBSERVABILITY.md) | Inspector, timeline, replay, overlays |
| [.agent/SCENARIOS.md](.agent/SCENARIOS.md) | Predefined simulation configurations |
| [.agent/ROADMAP.md](.agent/ROADMAP.md) | Development phases and milestones |
| [.agent/CONTRIBUTING.md](.agent/CONTRIBUTING.md) | Code style, conventions, PR process |

---

## Project Structure

```
emergent-sim/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── engine/          # Rendering, input, camera, ECS loop
│   ├── simulation/      # World logic, agents, resources, events
│   ├── ai/              # Utility scoring, decision systems
│   ├── observability/   # Inspector, timeline, replay, overlays
│   └── scenarios/       # Preconfigured world setups
├── assets/
│   ├── meshes/
│   ├── materials/
│   ├── scenarios/       # RON scenario definitions
│   └── config/          # RON configuration files
├── .agent/
│   └── ...              # All architecture documentation
├── tests/
│   ├── simulation/
│   └── determinism/
└── .github/workflows/
    └── ci.yml           # GitHub Actions CI
```

---

## Why Rust + Bevy 0.15?

- **Rust** enforces correctness at compile time — no data races, no null, no unsafe memory in simulation-critical code
- **Bevy 0.15** provides a complete, modern runtime (ECS + rendering + input + asset system + scheduling) with excellent ergonomics
- The combination makes complex simulation state manageable, verifiable, and testable

---

## Development Roadmap

### Phase 0 — Foundation ✅
- [x] Cargo workspace with correct structure
- [x] Module directory setup
- [x] Plugin stubs for all 5 layers
- [x] rustfmt & clippy config
- [x] GitHub Actions CI

### Phase 1 — Engine Foundation
- [x] 3D scene setup, lighting
- [x] Orbit camera with mouse input
- [x] Fixed timestep loop
- [x] Basic egui panels
- [x] Input mapping

### Phase 2 — Simulation Core
- [x] Zone and ResourceNode entities
- [x] Spatial grid implementation
- [x] Scenario loading
- [x] Debug overlays

### Phase 3 — Agents
- [ ] Agent components and spawning
- [ ] Needs degradation system
- [ ] Agent death mechanics
- [ ] Random movement

### Phase 4 — AI
- [ ] Utility scoring functions
- [ ] Decision pipeline
- [ ] Agent state transitions
- [ ] Resource consumption

### Phase 5 — Observability
- [ ] Entity inspector panel
- [ ] Event timeline
- [ ] Replay recording/playback
- [ ] All debug overlays

### Phase 6 — Scenarios & Polish
- [ ] All built-in scenarios
- [ ] Scenario switching
- [ ] Determinism test suite
- [ ] Performance optimization

See [.agent/ROADMAP.md](.agent/ROADMAP.md) for detailed breakdown.

---

## Testing

```bash
# All tests
cargo test --all-features

# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Determinism tests specifically
cargo test determinism --verbose

# Format check
cargo fmt --all -- --check

# Lint check
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Portfolio Context

This project is built as a technical portfolio piece targeting **game engine**, **game systems**, and **real-time backend** engineering roles. It is intentionally scoped to demonstrate engineering depth over breadth — the goal is a well-architected, well-instrumented, and reproducible simulation core, not a playable product.

---

## Requirements

- Rust 1.78+
- Bevy 0.15
- OS: Linux, macOS, or Windows

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) file for details.

---

## Build & Run

```bash
# Development (debug, fast compile)
cargo run

# Release (optimized)
cargo run --release

# Headless mode (no rendering)
cargo run --no-default-features --features headless

# With all observability tools enabled
cargo run --all-features
```

---

**Status:** Phase 0 Complete. Architecture & foundation ready. Ready to begin Phase 1 (Engine Foundation).
