# AI System

> Utility-based agent decision making — architecture, scoring functions, and extensibility design.

---

## Philosophy

Agents in emergent-sim do **not** follow scripts or behavior trees with hardcoded transitions. Instead, every agent runs a **utility scoring pipeline** at each decision tick:

1. Perceive the local environment
2. Score all available actions based on current state
3. Select the highest-scoring action
4. Write the decision to `DecisionOutput`
5. The simulation layer executes the decision

This produces emergent behavior: an agent starving near a food source will reliably eat, but an agent with low energy and nearby food will weigh both needs and choose based on relative urgency. No case statement required.

---

## Decision Pipeline

```
Agent Entity
    │
    ▼
┌───────────────────────────────┐
│        Perception Phase       │
│  - query SpatialGrid          │
│  - find nearby entities       │
│  - find nearby zones          │
│  - check AgentMemory          │
└──────────────┬────────────────┘
               │ PerceptionData (local, not stored in ECS)
               ▼
┌───────────────────────────────┐
│        Scoring Phase          │
│  for each ActionKind:         │
│    score = Σ(weight × curve)  │
└──────────────┬────────────────┘
               │ Vec<(ActionKind, f32)>
               ▼
┌───────────────────────────────┐
│       Selection Phase         │
│  pick argmax(scores)          │
│  resolve target entity/pos    │
└──────────────┬────────────────┘
               │
               ▼
        DecisionOutput component
        (consumed by sim layer)
```

---

## Utility Scoring

Each action receives a score in `[0.0, 1.0]`. The score is a weighted sum of **input curves** applied to normalized agent state values.

### Input Curves

Curves transform raw values (0.0–1.0) into utility scores. Common curve types:

```rust
pub enum Curve {
    Linear,                        // f(x) = x
    Quadratic,                     // f(x) = x²  (slow until urgent)
    InverseQuadratic,              // f(x) = 1 - (1-x)²  (fast then slow)
    Threshold { cutoff: f32 },     // f(x) = 0 if x < cutoff, else 1
    Logistic { steepness: f32 },   // S-curve
}

impl Curve {
    pub fn evaluate(&self, x: f32) -> f32 { ... }
}
```

### Action Scoring Functions

Each action has a dedicated scoring function. All functions receive the same inputs:

```rust
pub struct ScoringContext<'a> {
    pub needs: &'a Needs,
    pub state: &'a AgentState,
    pub perception: &'a PerceptionData,
    pub config: &'a AIConfig,
    pub time: f32,
}
```

#### `score_eat(ctx)`

```
hunger_urgency  = Quadratic(ctx.needs.hunger)
food_available  = if ctx.perception.nearest_food.is_some() { 1.0 } else { 0.0 }
not_eating      = if ctx.state.current != Eating { 1.0 } else { 0.3 }  // avoid thrash

score = hunger_urgency × food_available × not_eating × weight_eat
```

#### `score_rest(ctx)`

```
fatigue_urgency  = Quadratic(ctx.needs.fatigue)
rest_available   = if ctx.perception.nearest_rest_zone.is_some() { 1.0 } else { 0.5 }
not_resting      = if ctx.state.current != Resting { 1.0 } else { 0.2 }

score = fatigue_urgency × rest_available × not_resting × weight_rest
```

#### `score_explore(ctx)`

```
boredom          = Linear(time_since_last_new_zone / explore_timeout)
energy_ok        = InverseQuadratic(1.0 - ctx.needs.energy)  // avoid exploring when tired
hunger_ok        = Threshold { cutoff: 0.7 }(1.0 - ctx.needs.hunger)  // don't explore when starving

score = boredom × energy_ok × hunger_ok × weight_explore
```

#### `score_collect(ctx)`

```
resource_visible = ctx.perception.visible_resources.len() > 0
not_critical     = Threshold { cutoff: 0.8 }(1.0 - ctx.needs.hunger)  // don't collect if starving
                                                                          // (eat instead)
score = resource_visible × not_critical × weight_collect
```

---

## Perception System

Agents do **not** have global world knowledge. They perceive only what is within `AIConfig.perception_radius`.

```rust
pub struct PerceptionData {
    pub nearest_food: Option<(Entity, Vec3, f32)>,   // entity, position, distance
    pub nearest_rest_zone: Option<(Entity, Vec3, f32)>,
    pub visible_resources: Vec<(Entity, Vec3, ResourceKind, f32)>,  // +amount
    pub nearby_agents: Vec<(Entity, Vec3)>,           // for future competition logic
    pub current_zone: Option<(Entity, ZoneKind)>,
}
```

This is built each decision tick from `SpatialGrid` queries. It is a **local struct** — never stored in ECS.

---

## Decision Interval

Agents do not re-evaluate every tick (expensive and unrealistic). Decisions are re-evaluated every `AIConfig.decision_interval` seconds (default: `0.5s`).

Between decisions, agents continue their current action (e.g., keep walking toward the last target position).

Exception: a `NeedThresholdReached(Critical)` event forces an immediate re-evaluation for that agent.

---

## AIConfig — Tunable Weights

```rust
#[derive(Deserialize)]
pub struct UtilityWeights {
    pub eat: f32,      // default: 1.0
    pub rest: f32,     // default: 0.9
    pub explore: f32,  // default: 0.4
    pub collect: f32,  // default: 0.6
    pub idle: f32,     // default: 0.1  (fallback, always scores something)
}
```

These are loaded from `assets/config/ai.ron` and can be modified at runtime (scenario overrides).

---

## Emergent Behaviors Expected

The following dynamics should emerge from the utility system without being programmed explicitly:

| Behavior | Mechanism |
|---|---|
| Agents cluster near food when starving | High `score_eat`, food coordinates from memory/perception |
| Resting agents resist movement | Low scores on movement actions while resting |
| Agents explore when satisfied | `score_explore` dominates when needs are low |
| Resource competition | Multiple agents converge on same food node; first depletes it |
| Migration patterns | Depletion forces agents to explore → discover new zones |
| Starvation cycles | Scarcity scenario → `AgentDied` events cascade |

---

## Extensibility

To add a new action:

1. Add variant to `ActionKind` enum
2. Implement `score_newaction(ctx: &ScoringContext) -> f32`
3. Register function in the scoring pipeline
4. Add handler in `agent_state_transition_system` and relevant apply systems
5. Add weight to `UtilityWeights`

No other changes needed. The system is open for extension without modification of existing scoring logic.

---

## Future Extensions

- **Memory-weighted scoring** — remember last good food position, bias toward it
- **Social utility** — score actions based on nearby agent density (flocking / avoidance)
- **Threat response** — score `Flee` action based on proximity to hazard zones
- **Learning** — simple reinforcement signal (reward → adjust weights over time)

These are documented as future scope and are not part of the initial implementation target.
