# Decisions Log

> Architectural decisions, trade-offs considered, and rationale. Useful for understanding *why* the project is structured the way it is.

---

## ADR-001 — Use Bevy as the ECS runtime

**Decision:** Use Bevy 0.14 as the primary framework.

**Considered alternatives:**
- Custom ECS (hecs, legion, shipyard)
- Godot + GDExtension in Rust

**Rationale:**
- Bevy provides a complete runtime (ECS + rendering + input + asset system + scheduling) with strong ergonomics
- The project goal is to demonstrate system design, not ECS implementation
- Bevy's plugin system maps perfectly to the layer architecture
- `bevy-inspector-egui` provides the inspector foundation without building from scratch
- Good portfolio signal: Bevy is the dominant Rust game/simulation framework in 2024–2025

**Trade-offs accepted:**
- Bevy's API evolves quickly — version updates may require refactoring
- Less control than a custom ECS (acceptable for this project scope)

---

## ADR-002 — Utility-based AI over behavior trees

**Decision:** Implement AI as a utility scoring system, not a behavior tree.

**Considered alternatives:**
- Hardcoded state machine (simplest, least demonstrative)
- Behavior tree (well-known in game AI, but more rigid)
- Goal-Oriented Action Planning (GOAP, more complex)

**Rationale:**
- Utility scoring is more interesting to demonstrate than a state machine
- It produces genuine emergent behavior — agent "intelligence" emerges from weights, not scripted transitions
- The scoring functions are **pure functions**, making them easy to unit test
- Weights are externalized to config, making behavior tunable without code changes
- Simpler than GOAP while still demonstrating the key concept (context-sensitive action selection)

**Trade-offs accepted:**
- Utility scoring can produce "jittery" decisions if not smoothed (mitigated by decision interval)
- Less expressive than GOAP for complex multi-step planning (not needed at this scope)

---

## ADR-003 — Fixed timestep simulation with frame-rate rendering

**Decision:** Run simulation logic in `FixedUpdate`, render in `Update`.

**Rationale:**
- Simulation determinism requires a fixed timestep — floating-point math must produce identical results run-to-run
- Rendering at frame rate provides smooth visuals regardless of simulation speed
- Bevy's `FixedUpdate` schedule handles the interpolation between sim and render states

**Trade-offs accepted:**
- Visual positions may lag slightly behind simulation positions by up to one fixed-update interval
- Adds complexity of syncing simulation state to render components

**Mitigations:**
- Render components interpolate between last two simulation positions (smooth motion)
- `RenderSync` system runs every frame to pull updated positions from sim components

---

## ADR-004 — Spatial grid over k-d tree

**Decision:** Use a uniform grid for spatial partitioning.

**Considered alternatives:**
- k-d tree (dynamic, more efficient for non-uniform distributions)
- BVH (overkill for this entity count)
- No partitioning (brute force O(n²) checks)

**Rationale:**
- At the expected entity scale (< 500), a uniform grid is simpler and fast enough
- Grid is trivial to visualize as a debug overlay — adds observability value
- Grid rebuild cost is O(n) and predictable
- k-d tree would require either a library or significant implementation work with marginal benefit

**Trade-offs accepted:**
- Non-uniform distributions (all agents in one zone) cause cell hot spots
- Grid must be rebuilt every tick (vs. incrementally updated tree)
- Cell size must be tuned to world configuration

---

## ADR-005 — Event-driven cross-system communication

**Decision:** All cross-system communication uses Bevy's typed event system. No direct system calls, no shared mutable resources for signaling.

**Rationale:**
- Events enforce loose coupling between systems
- Any system can observe an event without modifying the producer
- Observability layer can listen to all events without touching simulation code
- Event history forms the foundation of the timeline tool

**Trade-offs accepted:**
- Events are fire-and-forget — responses cannot be awaited synchronously
- Events have a one-frame lifetime by default (must be buffered if timeline needs history)
- Slight overhead from event queue vs. direct mutation

---

## ADR-006 — RON for configuration assets

**Decision:** Use RON (Rusty Object Notation) for scenario and config files.

**Considered alternatives:**
- JSON (universal, no Rust types)
- TOML (readable, but awkward for nested structures)
- Custom format

**Rationale:**
- RON maps directly to Rust types via `serde` — no manual parsing
- Bevy's asset system has built-in RON support
- Enum variants in config are expressed naturally (`ZoneKind::Resource`)
- More readable than JSON for this use case

**Trade-offs accepted:**
- Less familiar than JSON for non-Rust collaborators (not a concern for this project)
- Limited tooling/editor support compared to JSON/TOML

---

## ADR-007 — Observability as a feature flag

**Decision:** All observability code compiles out via `#[cfg(feature = "observability")]`.

**Rationale:**
- Keeps release build clean of debug infrastructure
- Demonstrates understanding of Rust's conditional compilation
- Allows benchmarking the "pure" simulation loop without overhead
- Optional: headless mode uses `--no-default-features` and skips all UI

**Trade-offs accepted:**
- Feature flags add compile configuration complexity
- Must be careful not to let simulation-critical code hide behind the flag

---

## Open Questions

These are unresolved decisions deferred to later phases:

**Q: Should agents have persistent IDs across scene reloads?**
Current thinking: `AgentId` is a stable u64 assigned at spawn, survives entity reuse, but resets on full scenario reload.

**Q: Should the replay system use frame snapshots or event replay?**
Current thinking: both, selectable. Event replay is cheaper; frame snapshots enable rewind.

**Q: Should resource nodes have types beyond Food?**
Current thinking: Food only at MVP. Water/Material added in Phase 7 if scope allows.
