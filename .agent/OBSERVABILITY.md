# Observability

> Inspector, event timeline, replay system, and debug overlays — the instrumentation layer.

---

## Overview

Observability is a **first-class concern** in emergent-sim, not an afterthought. The ability to understand, inspect, and replay a running simulation is a core differentiator of this project.

All observability tools are:
- **Read-only** — they never write to simulation state
- **Toggleable** — individually enabled/disabled via `ObservabilityConfig`
- **Non-blocking** — their absence or presence does not affect simulation output
- **Determinism-safe** — they do not consume `SimRng` or affect event ordering

---

## 1. Simulation Inspector

A live ECS entity browser implemented with `bevy_egui` + `bevy-inspector-egui`.

### Features

- Browse all entities with their components
- Filter by component type (show only `Agent` entities, only `ResourceNode`, etc.)
- Click an entity to inspect all its component values
- Selected entity is tagged with `InspectorSelected` component → rendered with highlight

### Panel Layout

```
┌─ Entity Inspector ─────────────────────────────────────────────┐
│ Filter: [Agent ▼]  Search: [_________]       [42 entities]     │
├────────────────────────────────────────────────────────────────┤
│ ▶ Agent #001   Eating     hunger: 0.32  fatigue: 0.61          │
│ ▶ Agent #002   Exploring  hunger: 0.18  fatigue: 0.29          │
│ ● Agent #003   [SELECTED] hunger: 0.74  fatigue: 0.43          │
│   ├─ Transform    pos: (23.1, 0, 41.7)                         │
│   ├─ Needs        hunger: 0.74  fatigue: 0.43  energy: 0.55    │
│   ├─ AgentState   Eating  (was: MovingToTarget, 1.2s ago)      │
│   ├─ DecisionOutput  Eat  target: ResourceNode#14  score: 0.89 │
│   └─ AIDebugInfo  scores: Eat=0.89 Rest=0.32 Explore=0.11     │
├────────────────────────────────────────────────────────────────┤
│ ▶ ResourceNode #014  amount: 23.4 / 100.0  regen: 0.5/s        │
└────────────────────────────────────────────────────────────────┘
```

### Implementation Notes

- Uses `bevy-inspector-egui` for auto-derived component display
- Custom rendering for `Needs` (progress bars), `AIDebugInfo` (bar chart)
- Inspector does not affect simulation timing — runs in `Update` schedule, not `FixedUpdate`

---

## 2. Event Timeline

A chronological log of all simulation events, with visualization of causal chains.

### Features

- Logs all typed simulation events with timestamp and involved entities
- Filterable by event type
- Click an event entry → selects involved entity in inspector
- Visual heat map of event density over time
- Causal chain mode: trace from a single event backward/forward

### Panel Layout

```
┌─ Event Timeline ───────────────────────────────────────────────┐
│ Filter: [All ▼]  Time range: [0s ──────●────── 120s]           │
│                                                                 │
│ t=87.3s  ResourceDepleted   ResourceNode #014  (zone: Alpha)   │
│ t=87.1s  ResourceConsumed   Agent #003 ← ResourceNode #014     │
│ t=86.8s  AgentStateChanged  Agent #003  Exploring→Eating       │
│ t=86.2s  AgentStateChanged  Agent #003  MovingToTarget→Eating  │
│ t=85.4s  NeedThreshold      Agent #003  hunger=CRITICAL        │
│ t=82.1s  ResourceConsumed   Agent #007 ← ResourceNode #014     │
│                                                                 │
│ [▶ density graph ████░░░░██████████░░░████░░░░░░░ ]            │
└────────────────────────────────────────────────────────────────┘
```

### Storage

```rust
pub struct TimelineEvent {
    pub timestamp: f32,
    pub tick: u64,
    pub kind: TimelineEventKind,  // mirrors simulation events, flattened for display
    pub entities: Vec<Entity>,
}

pub struct EventTimeline {
    pub events: VecDeque<TimelineEvent>,  // bounded, oldest dropped after max_entries
    pub max_entries: usize,               // default: 1000
}
```

All `EventWriter`-published simulation events are mirrored to the timeline in `timeline_system` by reading `EventReader` for each event type.

---

## 3. Replay System

The simulation is fully reproducible. The replay system adds the ability to **record, rewind, and replay** a running simulation.

### Replay Modes

| Mode | Description |
|---|---|
| **Seed Replay** | Re-run simulation from scratch with the same seed. Fast, zero storage cost. Use for full reruns. |
| **Frame Replay** | Record state snapshots at configurable intervals. Supports rewind and scrubbing. Storage cost: proportional to recording length. |

### Frame Replay Recording

```rust
pub struct ReplayFrame {
    pub tick: u64,
    pub agent_snapshots: Vec<AgentSnapshot>,
    pub resource_snapshots: Vec<ResourceSnapshot>,
    pub events_this_frame: Vec<TimelineEvent>,
}

pub struct AgentSnapshot {
    pub id: AgentId,
    pub position: Vec3,
    pub needs: Needs,
    pub state: StateKind,
}
```

Recording interval: configurable (default every 10 ticks = ~167ms at 60hz). Memory budget: ~5MB for 5 minutes of simulation at default scale.

### Playback Controls

```
┌─ Replay Controls ──────────────────────────────────────────────┐
│                                                                 │
│  ◀◀ Rewind  ◀ Step  ■ Stop  ▶ Play  ▶ Step  ▶▶ Fast Forward   │
│                                                                 │
│  [0s ═══════════●══════════════════════════════ 120s]          │
│       t = 87.3s    Speed: [1× ▼]    Recording: ● REC           │
│                                                                 │
│  ⚠ Replay mode: simulation is paused                           │
└────────────────────────────────────────────────────────────────┘
```

### Determinism Contract

When using Seed Replay:
- `SimulationConfig.seed` is preserved
- All `SimRng` calls will produce identical sequences
- All events will occur at identical ticks
- Output is **bit-identical** to the original run

This is tested in `tests/determinism/`.

---

## 4. Debug Overlays

In-world 3D debug rendering using Bevy Gizmos.

### Available Overlays

#### Agent State Labels

Floating text above each agent showing:
- Current `StateKind`
- Hunger bar (color: green → yellow → red)
- Fatigue bar

Implemented as Bevy Gizmos lines + optional `bevy_text` billboard.

#### AI Score Visualization

For the selected agent, render a floating bar chart of current utility scores:

```
        [Agent #003]
         Eat: ████████░░ 0.89
        Rest: ████░░░░░░ 0.32
     Explore: ██░░░░░░░░ 0.11
```

Drawn using Bevy Gizmos 2D lines projected into 3D space.

#### Zone Radii

Wireframe circles showing zone boundaries and types:
- Blue = Rest zone
- Green = Resource zone
- Grey = Neutral zone

#### Spatial Grid

Optional: render the spatial grid cells as a wireframe. Highlights cells with > N entities (red tint). Useful for diagnosing clustering or grid sizing.

#### Agent Paths

For the selected agent: draw the last N positions as a fading trail (ghost positions).

### Overlay Toggle Keys

| Key | Overlay |
|---|---|
| `F3` | Toggle all overlays |
| `F4` | Toggle agent state labels |
| `F5` | Toggle AI score display |
| `F6` | Toggle zone radii |
| `F7` | Toggle spatial grid |

---

## ObservabilityConfig Resource

All observability features are controlled through a single resource, which is modifiable at runtime:

```rust
#[derive(Resource)]
pub struct ObservabilityConfig {
    pub inspector_open: bool,
    pub timeline_open: bool,
    pub overlays_enabled: bool,
    pub show_ai_scores: bool,        // only for InspectorSelected entity
    pub show_agent_state_labels: bool,
    pub show_zone_radii: bool,
    pub show_spatial_grid: bool,
    pub show_agent_paths: bool,
    pub timeline_max_entries: usize,
    pub replay_record_interval_ticks: u32,
}
```

In release builds, observability can be compiled out entirely via a feature flag:

```toml
[features]
default = ["observability"]
observability = ["bevy-inspector-egui", "bevy_egui"]
```

---

## Performance Budget

Observability tools have a strict performance budget to ensure they don't distort the simulation experience:

| Tool | Max CPU overhead |
|---|---|
| Inspector | < 0.5ms per frame |
| Timeline (record) | < 0.1ms per tick |
| Overlays | < 1ms per frame |
| Replay (record) | < 0.2ms per tick |

If any tool exceeds its budget consistently, it should be flagged as a performance issue.
