use dg4::geometry::{Polygon, Vec2};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke};

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
    polygon: Polygon,
}

impl Default for DgApp {
    fn default() -> Self {
        let mut app = Self {
            radius: 1.0,
            sides: 32,
            view_mode: ViewMode::Fit,
            zoom_px_per_unit: 120.0,
            polygon: Polygon::new(),
        };
        app.rebuild_polygon();
        app
    }
}

impl DgApp {
    fn rebuild_polygon(&mut self) {
        self.polygon = Polygon::regular_ngon(self.radius, self.sides);
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

                if ui.button("Reset").clicked() {
                    self.radius = 1.0;
                    self.sides = 32;
                    self.view_mode = ViewMode::Fit;
                    self.zoom_px_per_unit = 120.0;
                    changed = true;
                }

                if changed {
                    self.rebuild_polygon();
                }

                ui.separator();
                ui.label(format!("Vertices: {}", self.polygon.len()));
                ui.label(format!("Perimeter: {:.6}", self.polygon.perimeter()));
                if let Some(c) = self.polygon.centroid() {
                    ui.label(format!("Centroid: ({:.4}, {:.4})", c.x, c.y));
                }
            });

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
