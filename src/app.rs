use chrono::Utc;
use eframe::egui::{self, Label, RichText, Sense};
use egui_dock::{DockArea, DockState, NodeIndex};

use crate::{
    calc::{
        get_gmst, get_sun_position, get_terminator_outline, get_terminator_point,
        julian_date_from_unix_timestamp, AngleHour, EarthCoordsDeg, EarthCoordsRad, JulianDate,
        SunPosition,
    },
    init, MfColors, POPULATION_COUNT,
};

pub struct DaytimePopulationApp {
    tree: DockState<String>,
    context: AppContext,
}

#[derive(Default)]
struct AppContext {
    prev_sun_loc: Option<EarthCoordsDeg>,
    population_under_sun: u64,
    timestamp: i64,
    jd: JulianDate,
    sun_position: SunPosition,
    gp: EarthCoordsRad,
    gp_deg: EarthCoordsDeg,
    gmst: AngleHour,
    terminator_outline: Vec<f64>,
    sun_is_north: bool,
}

impl Default for DaytimePopulationApp {
    fn default() -> Self {
        let mut tree = DockState::new(vec!["Population".to_owned()]);
        let [a, _] =
            tree.main_surface_mut()
                .split_below(NodeIndex::root(), 0.3, vec!["Graph".to_owned()]);
        let [_, _] = tree
            .main_surface_mut()
            .split_right(a, 0.3, vec!["Data".to_owned()]);

        Self {
            tree,
            context: AppContext::default(),
        }
    }
}

impl DaytimePopulationApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        init::setup_custom_fonts(&cc.egui_ctx);
        init::setup_custom_styles(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Default::default()
    }
}

impl eframe::App for DaytimePopulationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label("Daylight Population Counter");
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.add(egui::Hyperlink::from_label_and_url(
                            RichText::new("Micfong").color(MfColors::BLUE_300),
                            "https://micfong.space/",
                        ));
                        ui.label("By");
                    });
                    ui.separator();

                    ui.add(egui::Hyperlink::from_label_and_url(
                        RichText::new("Source code").color(MfColors::BLUE_300),
                        "https://github.com/micfong-z/daylight_population",
                    ));
                    ui.separator();
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |_ui| {
            ctx.style_mut(|style| {
                style.interaction.tooltip_delay = 0.0;
            });

            DockArea::new(&mut self.tree)
                .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(true)
                .show_close_buttons(false)
                .show(ctx, &mut self.context);
        });

        ctx.request_repaint();
    }
}

impl egui_dock::TabViewer for AppContext {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        self.calculate();

        match tab.as_str() {
            "Graph" => self.graph(ui),
            "Data" => self.data(ui),
            "Population" => self.population(ui),
            _ => {
                ui.label("There is nothing here...\nYou see this because of a bug. Please report this to Micfong.");
            }
        }
    }
}

impl AppContext {
    fn graph(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .rounding(ui.visuals().widgets.noninteractive.rounding)
            .show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::Vec2::new(720.0, 360.0), Sense::hover());
                let rect_transform = egui::emath::RectTransform::from_to(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, rect.size()),
                    rect,
                );
                egui::Image::new(egui::include_image!("../assets/population_density.png"))
                    .fit_to_original_size(0.5)
                    .paint_at(ui, rect);

                let mut prev_point = egui::Pos2::ZERO;
                for (i, &lat) in self.terminator_outline.iter().enumerate() {
                    let x = (i as f32) / 2.0;
                    let y = (lat as f32) * 2.0 + 180.0;
                    let cur_point = rect_transform.transform_pos(egui::pos2(x, y));
                    if i != 0 {
                        ui.painter().line_segment(
                            [prev_point, cur_point],
                            egui::Stroke::new(0.5, MfColors::YELLOW_500),
                        );
                    }
                    prev_point = cur_point;
                }

                let sun_coords = rect_transform.transform_pos(egui::Pos2::new(
                    (self.gp_deg.lon * 2.0 + 360.0) as f32,
                    (self.gp_deg.lat * 2.0 + 180.0) as f32,
                ));
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        sun_coords + egui::vec2(0.0, -4.0),
                        sun_coords + egui::vec2(4.0, 0.0),
                        sun_coords + egui::vec2(0.0, 4.0),
                        sun_coords + egui::vec2(-4.0, 0.0),
                    ],
                    MfColors::YELLOW_500.gamma_multiply(0.25),
                    egui::Stroke::new(1.0, MfColors::YELLOW_500),
                ));

                if let Some(prev) = self.prev_sun_loc {
                    let position_delta = egui::vec2(
                        (self.gp_deg.lon - prev.lon) as f32,
                        (self.gp_deg.lat - prev.lat) as f32,
                    );
                    let arrow_delta = position_delta * 16.0 / position_delta.length();
                    ui.painter().arrow(
                        sun_coords,
                        arrow_delta,
                        egui::Stroke::new(1.0, MfColors::YELLOW_500),
                    );
                }

                self.prev_sun_loc = Some(self.gp_deg);
                if response.hovered() {
                    if let Some(mouse_pos) = response.hover_pos() {
                        let mouse_pos = rect_transform.inverse().transform_pos(mouse_pos);
                        let lat = (mouse_pos.y - 180.0) / 2.0;
                        let lon = (mouse_pos.x - 360.0) / 2.0;
                        let terminator_lat =
                            get_terminator_point(lon.to_radians().into(), &self.gp);

                        let pop_x = (mouse_pos.x * 2.0) as usize;
                        let pop_y = (mouse_pos.y * 2.0) as usize;
                        let pop = POPULATION_COUNT.get().unwrap()[pop_x + pop_y * 1440];
                        response.show_tooltip_ui(|ui| {
                            ui.add(
                                Label::new(format!("Lat: {:.6}°", lat))
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            );
                            ui.add(
                                Label::new(format!("Lon: {:.6}°", lon))
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            );
                            ui.add(
                                Label::new(
                                    RichText::new(format!(
                                        "Terminator lat: {:.6}°",
                                        terminator_lat.to_degrees()
                                    ))
                                    .color(MfColors::YELLOW_500),
                                )
                                .wrap_mode(egui::TextWrapMode::Extend),
                            );
                            ui.separator();

                            let zone_lat = (lat * 4.0).floor() / 4.0;
                            let zone_lon = (lon * 4.0).floor() / 4.0;

                            ui.add(Label::new(format!(
                                "Zone lat: {:.2}° — {:.2}°",
                                zone_lat,
                                zone_lat + 0.25
                            )));
                            ui.add(Label::new(format!(
                                "Zone lon: {:.2}° — {:.2}°",
                                zone_lon,
                                zone_lon + 0.25
                            )));
                            ui.add(
                                Label::new(format!("Population in zone: {}", pop))
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            );
                            ui.separator();
                            if self.sun_is_north && lat >= terminator_lat.to_degrees() as f32 {
                                ui.add(Label::new("Day").wrap_mode(egui::TextWrapMode::Extend));
                            } else {
                                ui.add(Label::new("Night").wrap_mode(egui::TextWrapMode::Extend));
                            }
                        });
                    }
                }
            });
    }

    fn data(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("awake_count_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.label("Timestamp");
                ui.label(format!("{:?}ms", self.timestamp));
                ui.end_row();

                ui.label("Julian Date");
                ui.label(format!("{:.8}", self.jd));
                ui.end_row();

                ui.label("GMST");
                ui.label(format!(
                    "{:?}h {:?}m {:.3}s",
                    self.gmst.hour, self.gmst.minute, self.gmst.second
                ));
                ui.end_row();

                ui.label("Sun Position (RA)");
                ui.label(format!("{:.9} rad", self.sun_position.ra));
                ui.label(format!("{:.8}°", self.sun_position.ra.to_degrees()));
                let ra_hour: AngleHour = self.sun_position.ra.into();
                ui.label(format!(
                    "{:?}h {:?}m {:.4}s",
                    ra_hour.hour, ra_hour.minute, ra_hour.second
                ));
                ui.end_row();

                ui.label("Sun Position (Dec)");
                ui.label(format!("{:.9} rad", self.sun_position.dec));
                ui.label(format!("{:.8}°", self.sun_position.dec.to_degrees()));
                let dec_hour: AngleHour = self.sun_position.dec.into();
                ui.label(format!(
                    "{:?}h {:?}m {:.4}s",
                    dec_hour.hour, dec_hour.minute, dec_hour.second
                ));
                ui.end_row();

                ui.label("Sun Position (R)");
                ui.label(format!("{:.8}", self.sun_position.r));
                ui.end_row();

                ui.label("Sun Position (Geographic)");
                ui.label(format!("Lat {:.8}°", self.gp_deg.lat));
                ui.label(format!("Lon {:.8}°", self.gp_deg.lon));
                ui.end_row();
            });
    }

    fn population(&mut self, ui: &mut egui::Ui) {
        ui.label("Population Under Sun");
        ui.heading(format!("{:?}", self.population_under_sun));
    }

    fn calculate(&mut self) {
        self.timestamp = Utc::now().timestamp_millis();
        self.jd = julian_date_from_unix_timestamp(self.timestamp);
        self.sun_position = get_sun_position(self.jd);
        self.gp = EarthCoordsRad::from_ra_dec(self.sun_position.ra, self.sun_position.dec, self.jd);
        self.gp_deg = EarthCoordsDeg::from(self.gp);
        self.gmst = get_gmst(self.jd).into();
        self.terminator_outline = get_terminator_outline(&self.gp)
            .iter()
            .map(|&lat| lat.to_degrees())
            .collect::<Vec<f64>>();
        self.sun_is_north = self.gp.lat > get_terminator_point(self.gp.lon, &self.gp);
        self.population_under_sun = 0;
        for (lon, terminator_lat) in self.terminator_outline.iter().enumerate() {
            if self.sun_is_north {
                for (i, &pop) in POPULATION_COUNT
                    .get()
                    .unwrap()
                    .iter()
                    .skip(lon)
                    .step_by(1440)
                    .enumerate()
                {
                    if i as i64 <= (720.0 - (terminator_lat + 90.0) * 4.0) as i64 {
                        self.population_under_sun += pop;
                    } else {
                        break;
                    }
                }
            } else {
                for (i, &pop) in POPULATION_COUNT
                    .get()
                    .unwrap()
                    .iter()
                    .skip(lon)
                    .step_by(1440)
                    .enumerate()
                {
                    if i as i64 >= (720.0 - (terminator_lat + 90.0) * 4.0) as i64 {
                        self.population_under_sun += pop;
                    }
                }
            }
        }
    }
}
