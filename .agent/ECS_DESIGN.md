# ECS Design

> Components, systems, resources, and events — the complete ECS vocabulary of emergent-sim.

---

## Design Philosophy

In Bevy ECS, data lives in **components** attached to **entities**, logic lives in **systems**, global state lives in **resources**, and inter-system communication happens through **events**. This document defines the canonical set of each.

Rules:
- Components hold **data only** — no methods with logic
- Systems are **pure functions** over queries — no side effects outside of ECS mutations
- Resources hold **global state** — must be justified (if it belongs to an entity, use a component)
- Events are **fire-and-forget** — no event should have a mandatory response

---

## Components

### Core Transform & Physics

```rust
// Re-use Bevy's built-in Transform for position/rotation/scale
// No custom wrapper needed at this stage

// Velocity — separate from Transform, owned by simulation layer
#[derive(Component)]
pub struct Velocity {
    pub linear: Vec3,
}

// Collider tag (used by spatial system for proximity detection)
#[derive(Component)]
pub struct Collider {
    pub radius: f32,
}
```

---

### Agent Components

```rust
/// Core identity of an agent
#[derive(Component)]
pub struct Agent {
    pub id: AgentId,       // stable unique ID (not Entity — survives reuse)
    pub age: f32,          // simulation seconds since spawn
}

/// Physical and biological needs — degrade over time
#[derive(Component)]
pub struct Needs {
    pub hunger: f32,       // 0.0 = satisfied, 1.0 = critical
    pub fatigue: f32,      // 0.0 = rested, 1.0 = exhausted
    pub energy: f32,       // 0.0 = depleted, 1.0 = full
}

/// Internal state machine
#[derive(Component)]
pub struct AgentState {
    pub current: StateKind,
    pub previous: StateKind,
    pub time_in_state: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StateKind {
    Idle,
    MovingToTarget,
    Eating,
    Resting,
    Exploring,
    Fleeing,   // future: competition/threat response
}

/// Output of AI decision pipeline — written by AI layer, consumed by simulation
#[derive(Component)]
pub struct DecisionOutput {
    pub action: ActionKind,
    pub target: Option<Entity>,
    pub target_position: Option<Vec3>,
    pub score: f32,         // for debug display
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionKind {
    Idle,
    MoveTo,
    Eat,
    Rest,
    Explore,
    Collect,
}

/// Optional lightweight memory
#[derive(Component, Default)]
pub struct AgentMemory {
    pub last_known_food: Option<Vec3>,
    pub last_known_rest: Option<Vec3>,
    pub visited_zones: Vec<ZoneId>,  // bounded, oldest removed
}
```

---

### Resource Components

```rust
/// A harvestable resource node
#[derive(Component)]
pub struct ResourceNode {
    pub kind: ResourceKind,
    pub amount: f32,       // current supply (0.0 → max_amount)
    pub max_amount: f32,
    pub regen_rate: f32,   // units per simulation second
    pub is_depleted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceKind {
    Food,
    Water,    // future expansion
    Material, // future expansion
}

/// A zone with specific properties
#[derive(Component)]
pub struct Zone {
    pub id: ZoneId,
    pub kind: ZoneKind,
    pub radius: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZoneKind {
    Resource,
    Rest,
    Neutral,
    Hazard,   // future expansion
}
```

---

### Debug / Observability Components

```rust
/// Marks an entity as currently selected in the inspector
#[derive(Component)]
pub struct InspectorSelected;

/// Stores last N AI scoring results for this agent (debug overlay)
#[derive(Component, Default)]
pub struct AIDebugInfo {
    pub last_scores: Vec<(ActionKind, f32)>,
    pub last_decision_time: f32,
}

/// Attached to entities that should render a debug label
#[derive(Component)]
pub struct DebugLabel {
    pub text: String,
}
```

---

## Resources (Global State)

```rust
/// Keyboard bindings for engine-level actions
#[derive(Resource)]
pub struct InputMap {
    // internal: Vec<(KeyCode, EngineAction)>
}

/// Runtime toggle for the engine ground grid overlay
#[derive(Resource)]
pub struct DebugGridConfig {
    pub enabled: bool,
    pub cell_size: f32,
    pub size: f32,
}

/// Master simulation configuration (loaded from asset)
#[derive(Resource)]
pub struct SimulationConfig {
    pub world_size: Vec2,
    pub initial_agent_count: u32,
    pub initial_resource_count: u32,
    pub needs_decay_rates: NeedsDecayRates,
    pub spatial_grid_cell_size: f32,
    pub seed: u64,
    pub global_decay_multiplier: f32,
    pub global_regen_multiplier: f32,
    pub agent_move_speed: f32,
    pub random_walk_turn_secs: f32,
    pub agent_collider_radius: f32,
    pub agent_visual_height: f32,
}

/// AI configuration (hot-reloadable)
#[derive(Resource)]
pub struct AIConfig {
    pub utility_weights: UtilityWeights,
    pub decision_interval: f32,  // seconds between agent decisions
    pub perception_radius: f32,
}

pub struct UtilityWeights {
    pub eat: f32,
    pub rest: f32,
    pub explore: f32,
    pub collect: f32,
    pub idle: f32,
}

/// Global simulation clock (separate from Bevy's Time for determinism)
#[derive(Resource, Default)]
pub struct SimulationTime {
    pub elapsed: f32,
    pub tick: u64,
    pub paused: bool,
}

/// Spatial partitioning grid
#[derive(Resource)]
pub struct SpatialGrid {
    // internal: HashMap<GridCell, Vec<Entity>>
    // exposed: query methods only
}

/// Deterministic random number generator for simulation systems
#[derive(Resource)]
pub struct SimRng {
    // internal: ChaCha8Rng seeded from SimulationConfig.seed
}

/// Global simulation metrics (written by sim, read by observability)
#[derive(Resource, Default)]
pub struct SimulationMetrics {
    pub agent_count: u32,
    pub total_resource_available: f32,
    pub avg_hunger: f32,
    pub avg_fatigue: f32,
    pub events_this_tick: u32,
}

/// Replay recording buffer
#[derive(Resource, Default)]
pub struct ReplayBuffer {
    pub frames: Vec<ReplayFrame>,
    pub is_recording: bool,
    pub is_replaying: bool,
    pub playback_index: usize,
}

/// Observability feature flags (runtime toggleable)
#[derive(Resource)]
pub struct ObservabilityConfig {
    pub inspector_open: bool,
    pub timeline_open: bool,
    pub overlays_enabled: bool,
    pub show_ai_scores: bool,
    pub show_zone_radii: bool,
    pub show_spatial_grid: bool,
}
```

---

## Events

Events are the **only** mechanism for cross-system communication. No system should query another system's internal state directly.

```rust
/// Engine-level action produced from raw input
#[derive(Event)]
pub struct EngineActionEvent {
    pub action: EngineAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineAction {
    TogglePause,
    ResetSimulationTime,
    ToggleInspector,
    ToggleDebugGrid,
}

/// Agent's needs crossed a threshold
#[derive(Event)]
pub struct NeedThresholdReached {
    pub agent: Entity,
    pub need: NeedKind,
    pub level: ThresholdLevel,  // Warning, Critical
}

/// Agent consumed a resource
#[derive(Event)]
pub struct ResourceConsumed {
    pub agent: Entity,
    pub resource: Entity,
    pub amount: f32,
    pub kind: ResourceKind,
}

/// A resource node was fully depleted
#[derive(Event)]
pub struct ResourceDepleted {
    pub resource: Entity,
    pub position: Vec3,
    pub kind: ResourceKind,
}

/// A resource node regenerated to full
#[derive(Event)]
pub struct ResourceReplenished {
    pub resource: Entity,
}

/// Agent changed its state
#[derive(Event)]
pub struct AgentStateChanged {
    pub agent: Entity,
    pub from: StateKind,
    pub to: StateKind,
}

/// Agent spawned
#[derive(Event)]
pub struct AgentSpawned {
    pub agent: Entity,
    pub position: Vec3,
}

/// Agent died (needs hit critical zero)
#[derive(Event)]
pub struct AgentDied {
    pub agent: Entity,
    pub cause: DeathCause,
}

#[derive(Clone, Copy, Debug)]
pub enum DeathCause {
    Starvation,
    Exhaustion,
}

/// Player-triggered scenario override
#[derive(Event)]
pub struct ScenarioEvent {
    pub kind: ScenarioEventKind,
}

#[derive(Clone, Debug)]
pub enum ScenarioEventKind {
    SpawnAgents { count: u32, position: Vec3 },
    DepleteAllResources,
    SetNeedsDecayMultiplier(f32),
    LoadPreset(String),
}
```

---

## Systems Overview

### Engine Systems

| System | Schedule | Reads | Writes |
|---|---|---|---|
| `spawn_scene` | Startup | — | ground plane, lights, `AmbientLight` |
| `spawn_orbit_camera` | Startup | — | camera entity |
| `handle_keyboard_input` | Update | `ButtonInput<KeyCode>`, `InputMap` | `EngineActionEvent` |
| `apply_engine_actions` | Update | `EngineActionEvent` | `SimulationTime`, `DebugGridConfig` |
| `handle_camera_input` | Update | mouse input | `OrbitCamera`, `Transform` |
| `draw_debug_grid_system` | Update | `DebugGridConfig` | gizmos |
| `update_simulation_time` | FixedUpdate | — | `SimulationTime` |

### Simulation Systems

| System | Schedule | Reads | Writes |
|---|---|---|---|
| `needs_decay_system` | FixedUpdate (Pre) | `SimulationTime`, `SimulationConfig` | `Needs` |
| `resource_regen_system` | FixedUpdate (Pre) | `SimulationTime` | `ResourceNode` |
| `spatial_grid_update_system` | FixedUpdate (Pre) | `Transform` | `SpatialGrid` |
| `agent_state_transition_system` | FixedUpdate (Apply) | `DecisionOutput` | `AgentState` |
| `agent_movement_system` | FixedUpdate (Apply) | `DecisionOutput`, `AgentState` | `Transform`, `Velocity` |
| `resource_consume_system` | FixedUpdate (Apply) | `DecisionOutput`, `AgentState` | `Needs`, `ResourceNode` → events |
| `rest_recovery_system` | FixedUpdate (Apply) | `AgentState` | `Needs` |
| `agent_death_system` | FixedUpdate (Post) | `Needs` | despawn → `AgentDied` events |
| `metrics_update_system` | FixedUpdate (Post) | `Needs`, agent query | `SimulationMetrics` |

### AI Systems

| System | Schedule | Reads | Writes |
|---|---|---|---|
| `ai_perception_system` | FixedUpdate (AIDecision) | `Transform`, `SpatialGrid`, `ResourceNode`, `Zone` | `AIDebugInfo` |
| `ai_scoring_system` | FixedUpdate (AIDecision) | `Needs`, `AgentState`, perception data | `DecisionOutput`, `AIDebugInfo` |

### Observability Systems

| System | Schedule | Reads | Writes |
|---|---|---|---|
| `inspector_ui_system` | Update | all components (read) | `InspectorSelected` |
| `timeline_system` | Update | all events (read) | timeline buffer |
| `overlay_draw_system` | Update | `Transform`, `AIDebugInfo`, `Zone` | Bevy Gizmos |
| `replay_record_system` | FixedUpdate (Post) | snapshot of all relevant state | `ReplayBuffer` |

### Scenario Systems

| System | Schedule | Reads | Writes |
|---|---|---|---|
| `load_default_scenario_system` | Startup | `assets/scenarios/equilibrium.ron` | `ActiveScenario`, `SimulationConfig` |
| `spawn_active_scenario_system` | PostStartup | `ActiveScenario` | `Zone`, `ResourceNode`, `Collider`, visual meshes/materials |

---

## Entity Archetypes

| Archetype | Required Components |
|---|---|
| **Agent (Phase 3)** | `Agent`, `Needs`, `AgentState`, `Transform`, `Velocity`, `Collider` |
| **Agent (Phase 4+)** | + `DecisionOutput` |
| **Resource Node** | `ResourceNode`, `Transform`, `Collider` |
| **Zone** | `Zone`, `Transform` |
| **Agent (debug)** | + `AIDebugInfo`, optionally `AgentMemory` |

Archetypes are spawned via builder functions (not inline in systems) to keep spawn logic centralized.
