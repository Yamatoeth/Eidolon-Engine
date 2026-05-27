# Contributing

> Code conventions, architectural rules, and development workflow for emergent-sim.

---

## Code Style

This project uses `rustfmt` and `clippy` with strict settings. All code must pass before merge.

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

### Naming Conventions

| Construct | Convention | Example |
|---|---|---|
| Components | `PascalCase` noun | `AgentState`, `ResourceNode` |
| Systems | `snake_case` + `_system` suffix | `needs_decay_system` |
| Events | `PascalCase` past-tense noun | `ResourceDepleted`, `AgentDied` |
| Resources | `PascalCase` noun | `SimulationConfig`, `SpatialGrid` |
| Plugins | `PascalCase` + `Plugin` suffix | `SimulationPlugin`, `AIPlugin` |
| Enums | `PascalCase` variants | `StateKind::Idle`, `ActionKind::Eat` |
| Config fields | `snake_case` | `perception_radius`, `decay_rate` |

### Module Structure

Each module file follows this order:
1. `use` imports (std → external crates → internal, separated by blank lines)
2. Public type definitions (structs, enums)
3. `impl` blocks
4. System functions
5. Helper/private functions

### Comments

- All public types must have a doc comment (`///`)
- Systems must have a one-line doc comment explaining their purpose
- Non-obvious logic must have an inline comment
- No commented-out code in committed files

---

## Architectural Rules

These rules are **non-negotiable** and will block PR review if violated:

### Rule 1: Layer Isolation
```
// ❌ FORBIDDEN in src/simulation/
use crate::engine::RenderComponent;

// ❌ FORBIDDEN in src/engine/
use crate::simulation::AgentState;
```

Engine and simulation layers must not import each other. See ARCHITECTURE.md for the dependency graph.

### Rule 2: Simulation is Never Called Directly
```rust
// ❌ FORBIDDEN — direct function call across systems
fn my_system(mut other_system: ResMut<OtherSystem>) { ... }

// ✅ CORRECT — communicate through events
fn my_system(mut events: EventWriter<ResourceConsumed>) {
    events.send(ResourceConsumed { ... });
}
```

### Rule 3: No Logic in Components
```rust
// ❌ FORBIDDEN — logic in component
impl Needs {
    pub fn apply_decay(&mut self, rate: f32, dt: f32) { ... }
}

// ✅ CORRECT — logic in system
fn needs_decay_system(mut query: Query<&mut Needs>, time: Res<SimulationTime>) { ... }
```

### Rule 4: No Hardcoded Simulation Values
```rust
// ❌ FORBIDDEN
const HUNGER_DECAY: f32 = 0.02;

// ✅ CORRECT — in SimulationConfig / AIConfig
let decay = config.needs_decay_rates.hunger_per_sec;
```

Exception: engine-layer constants (window size, default camera distance) may be hardcoded.

### Rule 5: Observability Never Mutates Simulation
```rust
// ❌ FORBIDDEN in observability layer
fn inspector_system(mut agents: Query<&mut AgentState>) { ... }

// ✅ CORRECT — read-only
fn inspector_system(agents: Query<&AgentState>) { ... }
```

### Rule 6: SimRng is Simulation-Only
```rust
// ❌ FORBIDDEN in observability, engine, or scenarios
fn my_system(mut rng: ResMut<SimRng>) { ... }

// SimRng must only be consumed in simulation and AI systems
```

---

## System Design Checklist

Before writing a new system, answer these:

- [ ] Which `SystemSet` does this belong to? (see ARCHITECTURE.md ordering)
- [ ] Does it read or write simulation state?
- [ ] If it produces output, should it use events or component writes?
- [ ] Is any value hardcoded that should come from config?
- [ ] Does it need to run in `FixedUpdate` (determinism) or `Update` (display)?
- [ ] Will it work correctly during replay playback?

---

## Adding a New Component

1. Define in the appropriate module (`simulation/`, `ai/`, etc.)
2. Add doc comment
3. Register in the plugin with `app.register_type::<MyComponent>()` (for inspector support)
4. Add to the relevant entity archetype in ECS_DESIGN.md
5. If it needs to be snapshotted for replay: add a `Snapshot` struct and update `ReplayFrame`

---

## Adding a New System

1. Write function with `_system` suffix
2. Place in correct module
3. Add doc comment
4. Register in plugin with correct `SystemSet` label
5. Add to the systems table in ECS_DESIGN.md
6. Write at least one unit test if it contains non-trivial logic

---

## Testing

### Unit Tests

Write unit tests for:
- Utility scoring functions (pure functions, easy to test)
- Needs decay calculations
- Spatial grid queries
- Curve evaluation functions

```bash
cargo test
```

### Determinism Tests

The most important test suite. Located in `tests/determinism/`:

```rust
#[test]
fn simulation_is_deterministic() {
    let output_a = run_headless_simulation(seed: 42, ticks: 1000);
    let output_b = run_headless_simulation(seed: 42, ticks: 1000);
    assert_eq!(output_a, output_b);
}
```

These tests run the simulation in headless mode (no rendering) and compare full state snapshots at specific ticks. Any non-determinism is a blocking bug.

### Integration Tests

Located in `tests/simulation/`:
- Verify agents eventually eat when hungry
- Verify resources deplete and regenerate
- Verify agents die when needs hit 1.0
- Verify scenario loading resets state correctly

---

## Git Workflow

### Branch Naming
- Features: `feat/phase-N-description`
- Bugfixes: `fix/short-description`
- Documentation: `docs/short-description`
- Refactoring: `refactor/short-description`

### Commit Messages
Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(ai): implement utility scoring for eat and rest actions
fix(simulation): resource regen no longer runs when depleted
docs(ecs): add AgentMemory component spec
refactor(spatial): replace Vec with SmallVec in grid cells
```

### PR Requirements
- CI passes (build + clippy + test)
- No layer isolation violations
- No hardcoded simulation values
- Doc comments on all new public types
- ARCHITECTURE/ECS_DESIGN.md updated if new components or systems added

---

## Development Tips

### Running with hot reload

Bevy's asset hot reloading works for `.ron` config files:
```bash
BEVY_ASSET_ROOT=. cargo run
```
Edit `assets/config/ai.ron` and see utility weights change live (Phase 6+).

### Profile with Tracy

```bash
cargo run --release --features bevy/trace_tracy
```

### Run headless (no window)

```bash
cargo run --no-default-features --features headless
```

Useful for benchmarking the simulation loop without rendering overhead.
