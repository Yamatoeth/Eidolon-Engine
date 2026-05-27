//! Spatial grid, proximity queries, chunk management.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::simulation::world::SimulationConfig;

/// Circular collider used by the spatial grid.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Collider {
    /// Radius in X/Z world units.
    pub radius: f32,
}

/// Integer coordinate for a spatial grid cell.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GridCell {
    /// X cell coordinate.
    pub x: i32,
    /// Z cell coordinate.
    pub z: i32,
}

/// Uniform spatial partition for proximity lookups.
#[derive(Resource, Debug, Clone)]
pub struct SpatialGrid {
    cells: HashMap<GridCell, Vec<Entity>>,
    cell_size: f32,
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self::new(10.0)
    }
}

impl SpatialGrid {
    /// Create an empty grid with the provided cell size.
    #[must_use]
    pub fn new(cell_size: f32) -> Self {
        Self {
            cells: HashMap::new(),
            cell_size: cell_size.max(0.1),
        }
    }

    /// Return the current cell size.
    #[must_use]
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    /// Clear all tracked entities and update the cell size.
    pub fn clear_with_cell_size(&mut self, cell_size: f32) {
        self.cells.clear();
        self.cell_size = cell_size.max(0.1);
    }

    /// Compute the cell containing the provided position.
    #[must_use]
    pub fn cell_for_position(&self, position: Vec3) -> GridCell {
        GridCell {
            x: (position.x / self.cell_size).floor() as i32,
            z: (position.z / self.cell_size).floor() as i32,
        }
    }

    /// Insert an entity into the grid by position.
    pub fn insert(&mut self, entity: Entity, position: Vec3) {
        let cell = self.cell_for_position(position);
        self.cells.entry(cell).or_default().push(entity);
    }

    /// Return entities in cells intersecting the query radius.
    #[must_use]
    pub fn entities_in_radius(&self, position: Vec3, radius: f32) -> Vec<Entity> {
        let min_cell = self.cell_for_position(position - Vec3::splat(radius));
        let max_cell = self.cell_for_position(position + Vec3::splat(radius));
        let mut entities = Vec::new();

        for x in min_cell.x..=max_cell.x {
            for z in min_cell.z..=max_cell.z {
                if let Some(cell_entities) = self.cells.get(&GridCell { x, z }) {
                    entities.extend(cell_entities.iter().copied());
                }
            }
        }

        entities
    }

    /// Iterate over populated cells.
    pub fn populated_cells(&self) -> impl Iterator<Item = (GridCell, &[Entity])> {
        self.cells
            .iter()
            .map(|(cell, entities)| (*cell, entities.as_slice()))
    }

    /// Return the number of populated cells.
    #[must_use]
    pub fn populated_cell_count(&self) -> usize {
        self.cells.len()
    }
}

/// Rebuild the spatial grid from collider-bearing entities.
pub fn spatial_grid_update_system(
    config: Res<SimulationConfig>,
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<Collider>>,
) {
    grid.clear_with_cell_size(config.spatial_grid_cell_size);

    for (entity, transform) in &query {
        grid.insert(entity, transform.translation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_map_to_expected_cells() {
        let grid = SpatialGrid::new(10.0);

        assert_eq!(
            grid.cell_for_position(Vec3::new(0.0, 0.0, 0.0)),
            GridCell { x: 0, z: 0 }
        );
        assert_eq!(
            grid.cell_for_position(Vec3::new(19.9, 0.0, -0.1)),
            GridCell { x: 1, z: -1 }
        );
    }

    #[test]
    fn radius_query_returns_neighboring_cells() {
        let mut grid = SpatialGrid::new(10.0);
        let near = Entity::from_raw(1);
        let far = Entity::from_raw(2);

        grid.insert(near, Vec3::new(0.0, 0.0, 0.0));
        grid.insert(far, Vec3::new(40.0, 0.0, 40.0));

        let found = grid.entities_in_radius(Vec3::ZERO, 12.0);

        assert!(found.contains(&near));
        assert!(!found.contains(&far));
    }
}
