//! emergent-sim — 3D ECS Simulation Engine
//!
//! A real-time 3D simulation engine built in Rust with Bevy, demonstrating
//! emergent behavior, utility-based AI, and advanced observability tooling.

pub mod ai;
pub mod engine;
pub mod observability;
pub mod scenarios;
pub mod simulation;

pub use ai::AIPlugin;
pub use engine::EnginePlugin;
pub use observability::ObservabilityPlugin;
pub use scenarios::ScenariosPlugin;
pub use simulation::SimulationPlugin;
