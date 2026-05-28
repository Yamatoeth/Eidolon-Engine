# Roadmap

> Phased development plan — from minimal skeleton to full demonstration-ready simulation engine.

---

## Guiding Principle

Each phase must produce a **working, demonstrable state**. No phase ends with broken code. The goal is incremental, always-shippable development.

---

## Phase 0 — Project Foundation
**Goal:** Compiling project with correct structure and dependencies.

### Tasks

- [ ] Init Cargo workspace with `emergent-sim` binary crate
- [ ] Configure `Cargo.toml` with Bevy, bevy-inspector-egui, rand, serde, ron dependencies
- [ ] Create full module directory structure (`engine/`, `simulation/`, `ai/`, `observability/`, `scenarios/`)
- [ ] Stub all plugins with empty `Plugin` impl
- [ ] Configure asset directory structure
- [ ] Set up `rustfmt.toml` and `clippy.toml` with project-wide style rules
- [ ] Write `README.md` draft
- [ ] Set up GitHub Actions CI (build + clippy + test)

**Deliverable:** `cargo run` opens a blank Bevy window. All module stubs compile.

---

## Phase 1 — Engine Foundation
**Goal:** A navigable 3D environment with a camera and basic rendering setup.

### Tasks

- [x] 3D scene setup (directional light, ambient light, shadows)
- [x] Orbit camera with mouse input (click-drag rotate, scroll zoom)
- [x] Flat world plane with material
- [x] Debug grid overlay (toggle)
- [x] Basic egui panel infrastructure (empty panels, toggleable)
- [x] Fixed timestep loop configured
- [x] `SimulationTime` resource with pause/resume
- [x] Input mapping table (keys → actions enum)

**Deliverable:** Navigate a 3D scene with camera. Pause key works. Empty inspector panel opens.

---

## Phase 2 — Simulation Core (Static World)
**Goal:** World, zones, and resource nodes exist as ECS entities.

### Tasks

- [x] `Zone` component + entity spawning
- [x] `ResourceNode` component + entity spawning
- [x] `SimulationConfig` resource loaded from RON asset
- [x] Zone visual representation (colored flat discs)
- [x] Resource node visual representation (simple meshes)
- [x] `SpatialGrid` resource with update system
- [x] Scenario loader — load `equilibrium.ron` by default
- [x] Debug overlay: zone radii wireframes
- [x] Debug overlay: spatial grid visualization

**Deliverable:** Load a scenario → see zones and resource nodes in 3D. Spatial grid updates when objects exist.

---

## Phase 3 — Agents (Dumb)
**Goal:** Agents exist, have needs, move randomly, die of starvation.

### Tasks

- [x] `Agent`, `Needs`, `AgentState`, `Velocity` components
- [x] Agent entity spawning (from `SimulationConfig`)
- [x] Agent visual (capsule mesh + color by state)
- [x] `needs_decay_system` — hunger/fatigue degrade over time
- [x] `AgentDied` event + entity despawn
- [x] `NeedThresholdReached` event
- [x] Random movement system (no AI yet — random walk)
- [x] Agent count display in UI
- [x] Debug overlay: agent state labels (hunger bar above head)
- [x] Inspector panel: list agents with their needs values

**Deliverable:** Agents wander randomly. They starve and disappear. You can watch their needs bars deplete in real time.

---

## Phase 4 — Basic AI (Utility Scoring)
**Goal:** Agents find and eat food, find rest. Emergent behavior begins.

### Tasks

- [x] `AIConfig` resource (weights, perception radius, decision interval)
- [x] `PerceptionData` struct + `ai_perception_system`
- [x] `AIDebugInfo` component
- [x] Utility scoring functions: `score_eat`, `score_rest`, `score_explore`, `score_idle`
- [x] `ai_scoring_system` → writes `DecisionOutput`
- [x] `agent_state_transition_system` → reads `DecisionOutput`
- [x] `agent_movement_system` — move toward `DecisionOutput.target_position`
- [x] `resource_consume_system` — consume resource when agent arrives
- [x] `resource_regen_system` — resources slowly refill
- [x] `ResourceConsumed`, `ResourceDepleted`, `ResourceReplenished` events
- [x] Debug overlay: AI score bars for selected agent
- [x] Inspector: show `DecisionOutput` with action and score

**Deliverable:** Agents navigate to food, eat, seek rest zones. Clear emergent clustering around resources when hungry.

---

## Phase 5 — Observability Layer
**Goal:** Full inspector, event timeline, and debug overlays functional.

### Tasks

- [ ] Inspector panel: component filter, entity list, detail view
- [ ] Custom egui widgets: needs progress bars, AI score bar chart
- [ ] `InspectorSelected` component + 3D highlight rendering
- [ ] Event timeline panel: event log with timestamps
- [ ] Timeline filter by event type
- [ ] Timeline density graph
- [ ] `EventTimeline` resource with rolling buffer
- [ ] All simulation events wired to timeline
- [ ] Replay recording system (`ReplayBuffer`)
- [ ] Replay playback controls UI
- [ ] Seed replay (restart from same seed)
- [ ] Frame replay scrubbing
- [ ] All debug overlays from OBSERVABILITY.md

**Deliverable:** Full observability suite functional. Can inspect any entity, view event history, replay any run.

---

## Phase 6 — Scenarios & Polish
**Goal:** All built-in scenarios functional. Demo-ready state.

### Tasks

- [ ] Scenario selector UI panel
- [ ] Scenario loading/switching without restart
- [ ] `scarcity.ron` — collapse scenario
- [ ] `overpopulation.ron` — migration scenario
- [ ] `stress_test.ron` — benchmark scenario
- [ ] `island.ron` — concentrated competition scenario
- [ ] Timed scenario events system
- [ ] Player interaction: click-to-inspect entity in 3D viewport
- [ ] Player interaction: spawn agent at clicked position
- [ ] Basic UI polish (fonts, layout, color theme)
- [ ] Performance profiling pass — meet observability budget targets
- [ ] Determinism test suite in `tests/determinism/`

**Deliverable:** All scenarios runnable. Full demo circuit possible. Project is portfolio-ready.

---

## Phase 7 — Extensions (Post-MVP)
**Goal:** Depth additions that improve the technical story.

These are not required for portfolio readiness but add significant engineering depth.

### Candidates (prioritize as desired)

- [ ] **Agent memory** — `AgentMemory` component, memory-weighted utility scoring
- [ ] **Social utility** — score based on nearby agent density (flocking / avoidance)
- [ ] **Terrain height** — non-flat world, pathfinding implications
- [ ] **Pathfinding** — grid-based A* or flow fields for obstacle avoidance
- [ ] **Hot-reload AI config** — modify utility weights at runtime without restart
- [ ] **Metrics export** — write simulation stats to CSV for external analysis
- [ ] **WASM build** — run simulation in browser for portfolio page
- [ ] **Headless mode** — run simulation without rendering (benchmark/test use)
- [ ] **Custom scenario editor** — in-app zone/resource placement tool

---

## Milestones Summary

| Phase | Name | Deliverable |
|---|---|---|
| 0 | Foundation | Compiles, structure in place |
| 1 | Engine | 3D scene, camera, pause |
| 2 | World | Zones, resources, scenario loading |
| 3 | Agents | Agents exist, needs, death |
| 4 | AI | Utility scoring, eating, resting |
| 5 | Observability | Inspector, timeline, replay |
| 6 | Scenarios | All presets, interaction, polish |
| 7 | Extensions | Depth features (ongoing) |

---

## Time Estimates

Very rough estimates assuming part-time development (evenings/weekends):

| Phase | Estimated Time |
|---|---|
| 0–1 | 1–2 weeks |
| 2–3 | 2–3 weeks |
| 4 | 2–3 weeks |
| 5 | 3–4 weeks |
| 6 | 2–3 weeks |
| Total (MVP) | ~12–15 weeks |
