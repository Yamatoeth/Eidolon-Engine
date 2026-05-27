# Contributing to emergent-sim

Thank you for your interest in contributing! This document outlines the development process, coding standards, and architectural rules.

## Code Quality Standards

All code must pass:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

### Rust Version

- Minimum: Rust 1.78
- The project uses stable Rust (no nightly required)
- Update `Cargo.toml` if changing MSRV

## Architecture Rules (MUST FOLLOW)

These are enforced in code review. Violations block PR approval.

### Rule 1: NO Upward Dependencies
```rust
// ❌ FORBIDDEN in src/simulation/
use crate::engine::RenderComponent;

// ❌ FORBIDDEN in src/engine/
use crate::simulation::AgentState;
```

The layer hierarchy is strict:
```
engine      → (no other layers)
simulation  → (no engine)
ai          → simulation (read-only)
observability → all (read-only)
scenarios   → simulation
```

### Rule 2: Events Only for Cross-System Communication
```rust
// ❌ FORBIDDEN — direct system calls
fn my_system(mut other_system: ResMut<OtherSystem>) { ... }

// ✅ CORRECT — events
fn my_system(mut events: EventWriter<ResourceConsumed>) {
    events.send(ResourceConsumed { ... });
}
```

### Rule 3: No Logic in Components
```rust
// ❌ FORBIDDEN
impl Needs {
    pub fn apply_decay(&mut self, rate: f32, dt: f32) { ... }
}

// ✅ CORRECT — in systems
fn needs_decay_system(mut query: Query<&mut Needs>, ...) { ... }
```

### Rule 4: No Hardcoded Simulation Values
```rust
// ❌ FORBIDDEN
const HUNGER_DECAY: f32 = 0.02;

// ✅ CORRECT
let decay = config.needs_decay_rates.hunger_per_sec;
```

Exception: Engine-layer constants (window size, camera defaults) may be hardcoded.

### Rule 5: Observability Never Mutates Simulation
```rust
// ❌ FORBIDDEN in observability layer
fn inspector_system(mut agents: Query<&mut AgentState>) { ... }

// ✅ CORRECT — read-only
fn inspector_system(agents: Query<&AgentState>) { ... }
```

### Rule 6: SimRng is Simulation-Only
`SimRng` must only be consumed in `src/simulation/` and `src/ai/`.

## Documentation Requirements

All public items must have doc comments:

```rust
/// Brief one-line description.
///
/// Optional longer explanation of purpose, design decisions,
/// or usage examples.
pub struct MyComponent {
    /// Field description
    pub field: f32,
}
```

All systems must have a one-line doc comment:

```rust
/// Decays all agent needs over time
fn needs_decay_system(...) { }
```

## Testing Expectations

### Unit Tests
- Utility scoring functions (pure, easy to test)
- Curve evaluation
- Needs calculations
- Spatial grid queries

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_eat_returns_zero_when_food_unavailable() {
        let ctx = make_context(food: None);
        assert_eq!(score_eat(&ctx), 0.0);
    }
}
```

### Determinism Tests
The most important test suite. Any non-determinism is a **blocking bug**.

```rust
#[test]
fn simulation_is_deterministic() {
    let output_a = run_headless(seed: 42, ticks: 1000);
    let output_b = run_headless(seed: 42, ticks: 1000);
    assert_eq!(output_a.snapshots, output_b.snapshots);
}
```

### Integration Tests
Verify high-level behaviors:
- Agents eat when hungry
- Resources deplete and regenerate
- Agents die when needs critical
- Scenarios load correctly

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(ai): implement utility scoring for eat action
fix(simulation): prevent agents dying before entering critical state
docs(ecs): add DecisionOutput component spec
refactor(spatial): optimize grid update to avoid redundant queries
test(determinism): add seed-replay test for Phase 4
```

### Format
- Type: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- Scope: module/feature name
- Subject: imperative present tense, lowercase, no period
- Body: explain *why*, not *what* (optional)

## Git Workflow

### Branch Naming
- Features: `feat/phase-N-description`
- Bugfixes: `fix/short-description`
- Documentation: `docs/short-description`
- Refactoring: `refactor/short-description`

### PR Requirements
- [ ] Branch is up-to-date with `main`
- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] All tests pass: `cargo test --all-features`
- [ ] New tests added for new functionality
- [ ] Architecture rules followed (no violations)
- [ ] Documentation updated if relevant
- [ ] Commit messages follow Conventional Commits

## Development Workflow

### Setting Up
```bash
git clone https://github.com/yourname/emergent-sim.git
cd emergent-sim
cargo build
```

### Before Submitting a PR
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test determinism --verbose
```

### Running Headless (Benchmark)
```bash
cargo run --no-default-features --features headless
```

### Hot Reloading Assets
```bash
BEVY_ASSET_ROOT=. cargo run
# Edit assets/config/ai.ron → changes reflect live
```

## Code Review Process

1. **Automated Checks** (GitHub Actions)
   - Build passes
   - Clippy clean
   - Tests pass
   - Determinism tests pass

2. **Manual Review**
   - Architecture rules enforced
   - Documentation adequate
   - Design sensible

3. **Approval**
   - At least one approval required
   - CI must pass

## Phases & Scope

Development is organized in phases. See [README.md](README.md) for the phase breakdown.

Each phase:
- Has a clear, working deliverable
- No phase ends with broken code
- Includes tests at appropriate levels
- Updates documentation

## Questions?

Refer to:
- [.agent/ARCHITECTURE.md](.agent/ARCHITECTURE.md) — System design
- [.agent/ECS_DESIGN.md](.agent/ECS_DESIGN.md) — Component/system specs
- [.agent/CONTRIBUTING.md](.agent/CONTRIBUTING.md) — In the doc folder
