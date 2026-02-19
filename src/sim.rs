use crate::geometry::{Polygon, Vec2};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct SimParams {
    pub edge_regularization_enabled: bool,
    pub target_edge_length: f64,
    pub edge_stiffness: f64,
    pub repulsion_enabled: bool,
    pub repulsion_radius: f64,
    pub repulsion_strength: f64,
    pub growth_enabled: bool,
    pub growth_rate: f64,
    pub split_enabled: bool,
    pub split_length: f64,
    pub jitter_enabled: bool,
    pub jitter_strength: f64,
}

#[derive(Debug)]
pub struct Simulation {
    polygon: Polygon,
    generation: u64,
    rng: StdRng,
}

impl Simulation {
    pub fn new(seed: u64) -> Self {
        Self {
            polygon: Polygon::new(),
            generation: 0,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn polygon(&self) -> &Polygon {
        &self.polygon
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn reset_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn rebuild_polygon(&mut self, radius: f64, sides: usize) {
        self.polygon = Polygon::regular_ngon(radius, sides);
        self.generation = 0;
    }

    pub fn step(&mut self, params: SimParams) {
        let n = self.polygon.len();
        if n == 0 {
            return;
        }

        // Snapshot positions so all forces are computed from the same state.
        let positions = self.polygon.vertices().to_vec();
        let mut delta = vec![Vec2::ZERO; n];

        if params.edge_regularization_enabled
            && params.edge_stiffness > 0.0
            && params.target_edge_length > 0.0
        {
            // Edge springs keep local spacing near target length.
            for i in 0..n {
                let j = (i + 1) % n;
                let d = positions[j] - positions[i];
                let len = d.length();
                if len > 1e-12 {
                    let dir = d / len;
                    let error = len - params.target_edge_length;
                    // Apply equal/opposite correction to edge endpoints.
                    let correction = dir * (error * params.edge_stiffness * 0.5);
                    delta[i] += correction;
                    delta[j] -= correction;
                }
            }
        }

        if params.repulsion_enabled && params.repulsion_strength > 0.0 && params.repulsion_radius > 0.0 {
            let radius_sq = params.repulsion_radius * params.repulsion_radius;
            // Pairwise repulsion excludes immediate polygon neighbors.
            for i in 0..n {
                for j in (i + 1)..n {
                    if are_neighbors(i, j, n) {
                        continue;
                    }

                    let d = positions[j] - positions[i];
                    let dist_sq = d.length_squared();
                    if dist_sq <= 1e-18 || dist_sq >= radius_sq {
                        continue;
                    }

                    let dist = dist_sq.sqrt();
                    let dir = d / dist;
                    // Repulsion fades linearly to zero at the radius boundary.
                    let proximity = 1.0 - dist / params.repulsion_radius;
                    let mag = params.repulsion_strength * proximity;
                    let push = dir * (mag * 0.5);

                    delta[i] -= push;
                    delta[j] += push;
                }
            }
        }

        if params.growth_enabled && params.growth_rate != 0.0 {
            let area = signed_area(&positions);
            let outward_sign = if area >= 0.0 { -1.0 } else { 1.0 };

            for i in 0..n {
                let prev = positions[(i + n - 1) % n];
                let next = positions[(i + 1) % n];
                let tangent = next - prev;
                let len = tangent.length();
                if len <= 1e-12 {
                    continue;
                }
                let dir = tangent / len;
                let normal = Vec2::new(dir.y, -dir.x) * outward_sign;
                delta[i] += normal * params.growth_rate;
            }
        }

        if params.jitter_enabled && params.jitter_strength > 0.0 {
            // Brownian term adds small random perturbation per vertex.
            for d in &mut delta {
                let jx = self.rng.gen_range(-1.0..1.0) * params.jitter_strength;
                let jy = self.rng.gen_range(-1.0..1.0) * params.jitter_strength;
                *d += Vec2::new(jx, jy);
            }
        }

        // Apply total displacement field to the polygon.
        for (v, d) in self.polygon.vertices_mut().iter_mut().zip(delta) {
            *v += d;
        }

        if params.split_enabled && params.split_length > 0.0 {
            let positions = self.polygon.vertices();
            if positions.len() >= 2 {
                let mut next_vertices = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    let a = positions[i];
                    let b = positions[(i + 1) % positions.len()];
                    next_vertices.push(a);

                    let len = a.distance(b);
                    if len > params.split_length {
                        let segments = (len / params.split_length).ceil() as usize;
                        if segments > 1 {
                            let denom = segments as f64;
                            for k in 1..segments {
                                let t = (k as f64) / denom;
                                next_vertices.push(a.lerp(b, t));
                            }
                        }
                    }
                }
                self.polygon.replace_vertices(next_vertices);
            }
        }

        self.generation = self.generation.saturating_add(1);
    }
}

pub fn regular_ngon_edge_length(radius: f64, sides: usize) -> f64 {
    if radius <= 0.0 || sides < 3 {
        return 0.0;
    }
    // Chord length of an inscribed regular n-gon.
    2.0 * radius * (PI / sides as f64).sin()
}

pub fn average_edge_length(polygon: &Polygon) -> f64 {
    let n = polygon.len();
    if n < 2 {
        return 0.0;
    }
    // Mean edge length for a closed polygon.
    polygon.perimeter() / n as f64
}

fn are_neighbors(i: usize, j: usize, n: usize) -> bool {
    if n < 2 || i == j {
        return true;
    }
    // Adjacent indices are connected by an edge in the closed loop.
    let next_i = (i + 1) % n;
    let prev_i = (i + n - 1) % n;
    j == next_i || j == prev_i
}

fn signed_area(points: &[Vec2]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }

    let mut sum = 0.0;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        sum += a.x * b.y - b.x * a.y;
    }
    0.5 * sum
}
