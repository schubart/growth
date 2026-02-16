use dg4::geometry::{Polygon, Vec2};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke};
use std::f64::consts::PI;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Differential Growth Playground",
        options,
        Box::new(|_cc| Ok(Box::new(DgApp::default()))),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Fit,
    FixedZoom,
}

impl ViewMode {
    fn label(self) -> &'static str {
        match self {
            Self::Fit => "Fit",
            Self::FixedZoom => "Fixed Zoom",
        }
    }
}

#[derive(Debug)]
struct DgApp {
    radius: f64,
    sides: usize,
    view_mode: ViewMode,
    zoom_px_per_unit: f64,
    edge_regularization_enabled: bool,
    target_edge_length: f64,
    edge_stiffness: f64,
    jitter_enabled: bool,
    jitter_strength: f64,
    auto_step: bool,
    steps_per_frame: usize,
    generation: u64,
    rng: SimpleRng,
    polygon: Polygon,
}

impl Default for DgApp {
    fn default() -> Self {
        let mut app = Self {
            radius: 1.0,
            sides: 32,
            view_mode: ViewMode::Fit,
            zoom_px_per_unit: 120.0,
            edge_regularization_enabled: true,
            target_edge_length: regular_ngon_edge_length(1.0, 32),
            edge_stiffness: 0.2,
            jitter_enabled: true,
            jitter_strength: 0.005,
            auto_step: false,
            steps_per_frame: 1,
            generation: 0,
            rng: SimpleRng::new(0xD1FF_EA11_2026_0001),
            polygon: Polygon::new(),
        };
        app.rebuild_polygon();
        app
    }
}

impl DgApp {
    fn rebuild_polygon(&mut self) {
        self.polygon = Polygon::regular_ngon(self.radius, self.sides);
        self.target_edge_length = regular_ngon_edge_length(self.radius, self.sides);
        self.generation = 0;
    }

    fn step_sim(&mut self) {
        let n = self.polygon.len();
        if n == 0 {
            return;
        }

        let positions = self.polygon.vertices().to_vec();
        let mut delta = vec![Vec2::ZERO; n];

        if self.edge_regularization_enabled && self.edge_stiffness > 0.0 && self.target_edge_length > 0.0
        {
            for i in 0..n {
                let j = (i + 1) % n;
                let d = positions[j] - positions[i];
                let len = d.length();
                if len > 1e-12 {
                    let dir = d / len;
                    let error = len - self.target_edge_length;
                    let correction = dir * (error * self.edge_stiffness * 0.5);
                    delta[i] += correction;
                    delta[j] -= correction;
                }
            }
        }

        if self.jitter_enabled && self.jitter_strength > 0.0 {
            for d in &mut delta {
                let jx = self.rng.next_signed_unit() * self.jitter_strength;
                let jy = self.rng.next_signed_unit() * self.jitter_strength;
                *d += Vec2::new(jx, jy);
            }
        }

        for (v, d) in self.polygon.vertices_mut().iter_mut().zip(delta) {
            *v += d;
        }

        self.generation = self.generation.saturating_add(1);
    }

    fn draw_polygon(ui: &mut egui::Ui, rect: Rect, polygon: &Polygon, view_mode: ViewMode, fixed_zoom: f64) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(20));

        if polygon.is_empty() {
            return;
        }

        let (min, max) = bounds(polygon.vertices()).unwrap_or((Vec2::ZERO, Vec2::ZERO));
        let center = (min + max) * 0.5;
        let width = (max.x - min.x).max(1e-6);
        let height = (max.y - min.y).max(1e-6);

        let scale = match view_mode {
            ViewMode::Fit => {
                let scale_x = rect.width() as f64 / width;
                let scale_y = rect.height() as f64 / height;
                scale_x.min(scale_y) * 0.9
            }
            ViewMode::FixedZoom => fixed_zoom.max(1.0),
        };

        let to_screen = |p: Vec2| -> Pos2 {
            let local = (p - center) * scale;
            Pos2::new(
                rect.center().x + local.x as f32,
                rect.center().y - local.y as f32,
            )
        };

        let mut points: Vec<Pos2> = polygon.vertices().iter().copied().map(to_screen).collect();
        if points.len() > 1 {
            points.push(points[0]);
            painter.add(Shape::line(points, Stroke::new(2.0, Color32::LIGHT_GREEN)));
        }

        for v in polygon.vertices() {
            painter.circle_filled(to_screen(*v), 3.0, Color32::from_rgb(250, 220, 130));
        }
    }
}

impl eframe::App for DgApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("controls")
            .resizable(false)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Starter Polygon");
                ui.separator();

                let mut changed = false;

                changed |= ui
                    .add(egui::Slider::new(&mut self.radius, 0.05..=10.0).text("Radius"))
                    .changed();

                changed |= ui
                    .add(egui::Slider::new(&mut self.sides, 3..=512).text("Sides"))
                    .changed();

                egui::ComboBox::from_label("View")
                    .selected_text(self.view_mode.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.view_mode, ViewMode::Fit, "Fit");
                        ui.selectable_value(&mut self.view_mode, ViewMode::FixedZoom, "Fixed Zoom");
                    });

                if self.view_mode == ViewMode::FixedZoom {
                    ui.add(
                        egui::Slider::new(&mut self.zoom_px_per_unit, 10.0..=400.0)
                            .text("Zoom (px/unit)"),
                    );
                }

                ui.separator();
                ui.heading("Simulation");
                ui.checkbox(&mut self.edge_regularization_enabled, "Edge Regularization");
                ui.add(
                    egui::Slider::new(&mut self.target_edge_length, 0.0001..=2.0)
                        .logarithmic(true)
                        .text("Target Edge Length"),
                );
                ui.add(
                    egui::Slider::new(&mut self.edge_stiffness, 0.0..=1.0).text("Edge Stiffness"),
                );
                if ui.button("Set Target From Current Shape").clicked() {
                    self.target_edge_length = average_edge_length(&self.polygon);
                }

                ui.separator();
                ui.checkbox(&mut self.jitter_enabled, "Brownian Jitter");
                ui.add(
                    egui::Slider::new(&mut self.jitter_strength, 0.0..=0.05)
                        .text("Jitter Strength"),
                );
                ui.add(egui::Slider::new(&mut self.steps_per_frame, 1..=32).text("Steps/Frame"));

                ui.horizontal(|ui| {
                    if ui.button("Step").clicked() {
                        self.step_sim();
                    }
                    ui.checkbox(&mut self.auto_step, "Run");
                });

                if ui.button("Reset").clicked() {
                    self.radius = 1.0;
                    self.sides = 32;
                    self.view_mode = ViewMode::Fit;
                    self.zoom_px_per_unit = 120.0;
                    self.edge_regularization_enabled = true;
                    self.edge_stiffness = 0.2;
                    self.jitter_enabled = true;
                    self.jitter_strength = 0.005;
                    self.auto_step = false;
                    self.steps_per_frame = 1;
                    self.rng = SimpleRng::new(0xD1FF_EA11_2026_0001);
                    changed = true;
                }

                if changed {
                    self.rebuild_polygon();
                }

                ui.separator();
                ui.label(format!("Vertices: {}", self.polygon.len()));
                ui.label(format!("Perimeter: {:.6}", self.polygon.perimeter()));
                ui.label(format!(
                    "Avg Edge Length: {:.6}",
                    average_edge_length(&self.polygon)
                ));
                ui.label(format!("Generation: {}", self.generation));
                if let Some(c) = self.polygon.centroid() {
                    ui.label(format!("Centroid: ({:.4}, {:.4})", c.x, c.y));
                }
            });

        if self.auto_step {
            for _ in 0..self.steps_per_frame {
                self.step_sim();
            }
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (response, _painter) = ui.allocate_painter(available, Sense::hover());
            Self::draw_polygon(ui, response.rect, &self.polygon, self.view_mode, self.zoom_px_per_unit);
        });
    }
}

fn bounds(points: &[Vec2]) -> Option<(Vec2, Vec2)> {
    if points.is_empty() {
        return None;
    }

    let mut min = points[0];
    let mut max = points[0];

    for p in points.iter().copied().skip(1) {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }

    Some((min, max))
}

fn regular_ngon_edge_length(radius: f64, sides: usize) -> f64 {
    if radius <= 0.0 || sides < 3 {
        return 0.0;
    }
    2.0 * radius * (PI / sides as f64).sin()
}

fn average_edge_length(polygon: &Polygon) -> f64 {
    let n = polygon.len();
    if n < 2 {
        return 0.0;
    }
    polygon.perimeter() / n as f64
}

#[derive(Debug, Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_unit(&mut self) -> f64 {
        const SCALE: f64 = (1u64 << 53) as f64;
        ((self.next_u64() >> 11) as f64) / SCALE
    }

    fn next_signed_unit(&mut self) -> f64 {
        self.next_unit() * 2.0 - 1.0
    }
}
