use dg4::geometry::{Polygon, Vec2};
use dg4::sim::{average_edge_length, regular_ngon_edge_length, SimParams, Simulation};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke};

// Launch a native egui desktop window.
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
    // Auto-fit polygon bounds into the viewport.
    Fit,
    // Use fixed pixels-per-unit so radius changes are visible.
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
    // Starter shape controls.
    radius: f64,
    sides: usize,
    // Camera / view controls.
    view_mode: ViewMode,
    zoom_px_per_unit: f64,
    // Edge spring force controls.
    edge_regularization_enabled: bool,
    target_edge_length: f64,
    edge_stiffness: f64,
    // Non-neighbor short-range repulsion controls.
    repulsion_enabled: bool,
    repulsion_radius: f64,
    repulsion_strength: f64,
    // Brownian jitter controls.
    jitter_enabled: bool,
    jitter_strength: f64,
    // Simulation stepping controls.
    auto_step: bool,
    steps_per_frame: usize,
    // Simulation state.
    sim: Simulation,
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
            repulsion_enabled: true,
            repulsion_radius: 0.15,
            repulsion_strength: 0.01,
            jitter_enabled: true,
            jitter_strength: 0.005,
            auto_step: false,
            steps_per_frame: 1,
            sim: Simulation::new(0xD1FF_EA11_2026_0001),
        };
        app.rebuild_polygon();
        app
    }
}

impl DgApp {
    // Rebuild starter geometry from current shape parameters.
    fn rebuild_polygon(&mut self) {
        self.sim.rebuild_polygon(self.radius, self.sides);
        self.target_edge_length = regular_ngon_edge_length(self.radius, self.sides);
    }

    fn sim_params(&self) -> SimParams {
        SimParams {
            edge_regularization_enabled: self.edge_regularization_enabled,
            target_edge_length: self.target_edge_length,
            edge_stiffness: self.edge_stiffness,
            repulsion_enabled: self.repulsion_enabled,
            repulsion_radius: self.repulsion_radius,
            repulsion_strength: self.repulsion_strength,
            jitter_enabled: self.jitter_enabled,
            jitter_strength: self.jitter_strength,
        }
    }

    // Draw polygon in viewport with either fit or fixed zoom mapping.
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
                // Preserve aspect ratio while fitting bounds with a small margin.
                let scale_x = rect.width() as f64 / width;
                let scale_y = rect.height() as f64 / height;
                scale_x.min(scale_y) * 0.9
            }
            ViewMode::FixedZoom => fixed_zoom.max(1.0),
        };

        // World-to-screen transform centered on polygon bounds.
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
        // Left panel exposes all simulation controls and metrics.
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
                    // Re-anchor target edge length to current geometry.
                    self.target_edge_length = average_edge_length(self.sim.polygon());
                }

                ui.separator();
                ui.checkbox(&mut self.repulsion_enabled, "Self Repulsion");
                ui.add(
                    egui::Slider::new(&mut self.repulsion_radius, 0.0001..=2.0)
                        .logarithmic(true)
                        .text("Repulsion Radius"),
                );
                ui.add(
                    egui::Slider::new(&mut self.repulsion_strength, 0.0..=0.1)
                        .text("Repulsion Strength"),
                );

                ui.separator();
                ui.checkbox(&mut self.jitter_enabled, "Brownian Jitter");
                ui.add(
                    egui::Slider::new(&mut self.jitter_strength, 0.0..=0.05)
                        .text("Jitter Strength"),
                );
                ui.add(egui::Slider::new(&mut self.steps_per_frame, 1..=32).text("Steps/Frame"));

                ui.horizontal(|ui| {
                    if ui.button("Step").clicked() {
                        self.sim.step(self.sim_params());
                    }
                    ui.checkbox(&mut self.auto_step, "Run");
                });

                if ui.button("Reset").clicked() {
                    // Reset controls and RNG seed to deterministic defaults.
                    self.radius = 1.0;
                    self.sides = 32;
                    self.view_mode = ViewMode::Fit;
                    self.zoom_px_per_unit = 120.0;
                    self.edge_regularization_enabled = true;
                    self.edge_stiffness = 0.2;
                    self.repulsion_enabled = true;
                    self.repulsion_radius = 0.15;
                    self.repulsion_strength = 0.01;
                    self.jitter_enabled = true;
                    self.jitter_strength = 0.005;
                    self.auto_step = false;
                    self.steps_per_frame = 1;
                    self.sim.reset_seed(0xD1FF_EA11_2026_0001);
                    changed = true;
                }

                if changed {
                    self.rebuild_polygon();
                }

                ui.separator();
                ui.label(format!("Vertices: {}", self.sim.polygon().len()));
                ui.label(format!("Perimeter: {:.6}", self.sim.polygon().perimeter()));
                ui.label(format!(
                    "Avg Edge Length: {:.6}",
                    average_edge_length(self.sim.polygon())
                ));
                ui.label(format!("Generation: {}", self.sim.generation()));
                if let Some(c) = self.sim.polygon().centroid() {
                    ui.label(format!("Centroid: ({:.4}, {:.4})", c.x, c.y));
                }
            });

        if self.auto_step {
            // Advance multiple steps per frame for faster evolution.
            for _ in 0..self.steps_per_frame {
                self.sim.step(self.sim_params());
            }
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (response, _painter) = ui.allocate_painter(available, Sense::hover());
            Self::draw_polygon(
                ui,
                response.rect,
                self.sim.polygon(),
                self.view_mode,
                self.zoom_px_per_unit,
            );
        });
    }
}

fn bounds(points: &[Vec2]) -> Option<(Vec2, Vec2)> {
    if points.is_empty() {
        return None;
    }

    // Axis-aligned bounding box over all vertices.
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
