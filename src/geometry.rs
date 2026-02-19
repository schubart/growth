use glam::DVec2;
use std::f64::consts::TAU;

pub type Real = f64;
pub type Vec2 = DVec2;

// Closed polygon represented as ordered vertices.
#[derive(Debug, Clone, Default)]
pub struct Polygon {
    vertices: Vec<Vec2>,
}

impl Polygon {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn replace_vertices(&mut self, vertices: Vec<Vec2>) {
        self.vertices = vertices;
    }

    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn vertices(&self) -> &[Vec2] {
        &self.vertices
    }

    pub fn vertices_mut(&mut self) -> &mut [Vec2] {
        &mut self.vertices
    }

    pub fn regular_ngon(radius: Real, sides: usize) -> Self {
        if sides < 3 || radius <= 0.0 {
            return Self::new();
        }

        // Evenly distribute vertices on a circle.
        let mut vertices = Vec::with_capacity(sides);
        for i in 0..sides {
            let t = TAU * (i as Real) / (sides as Real);
            vertices.push(Vec2::new(radius * t.cos(), radius * t.sin()));
        }
        Self { vertices }
    }

    pub fn perimeter(&self) -> Real {
        let n = self.vertices.len();
        if n < 2 {
            return 0.0;
        }

        // Sum edge lengths, including the closing edge.
        let mut total = 0.0;
        for i in 0..n {
            let a = self.vertices[i];
            let b = self.vertices[(i + 1) % n];
            total += a.distance(b);
        }
        total
    }

    pub fn centroid(&self) -> Option<Vec2> {
        if self.vertices.is_empty() {
            return None;
        }
        // Simple vertex-average centroid for current polyline model.
        let sum = self
            .vertices
            .iter()
            .copied()
            .fold(Vec2::ZERO, |acc, v| acc + v);
        Some(sum / (self.vertices.len() as Real))
    }
}
