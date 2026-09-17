use companion_core::{
    Battery, BluetoothCategory, CharacterPack, Decision, Director, PowerMode, State, Weather,
};
use companion_platform::{weather, SensorHub, Settings};
use eframe::egui::{
    self, Color32, Pos2, Rect, RichText, Sense, TextureHandle, Vec2, ViewportCommand, ViewportId,
};
use notify::Watcher;
use std::{
    collections::BTreeMap,
    sync::mpsc,
    time::{Duration, Instant},
};

const PLUM: Color32 = Color32::from_rgb(247, 249, 247);
const AMBER: Color32 = Color32::from_rgb(46, 115, 121);
const MUTED: Color32 = Color32::from_rgb(92, 108, 116);
const PAPER: Color32 = Color32::from_rgb(205, 225, 240);

#[derive(Clone, Copy)]
enum Action {
    Lab,
    TerminalOnly,
    Pause,
    ClickThrough,
    Reset,
    Scale(u32),
    Power(PowerMode),
    Quit,
}

pub fn run(pack: CharacterPack, mut settings: Settings, lab: bool, smoke: bool) -> eframe::Result {
    if smoke {
        settings.first_run = false;
    }
    let size = Vec2::new(
        pack.manifest.sprite_size[0] as f32,
        pack.manifest.sprite_size[1] as f32,
    ) * settings.scale as f32;
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Scopobot")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(false)
        .with_inner_size(size)
        .with_taskbar(false)
        .with_active(false);
    if let Some([x, y]) = settings.position {
        viewport = viewport.with_position([x, y]);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        vsync: false,
        multisampling: 0,
        ..Default::default()
    };
    eframe::run_native(
        "Scopobot",
        options,
        Box::new(move |cc| {
            let mut app = Companion::new(cc, pack, settings, lab);
            app.smoke = smoke;
            Ok(Box::new(app))
        }),
    )
}

struct Companion {
    smoke: bool,
    activity: Option<(String, Instant)>,
    hovered: bool,
    dance_party: bool,
    bubble: Option<(String, Instant)>,
    speech_visible: bool,
    dialogue_index: usize,

    pack: CharacterPack,
    textures: BTreeMap<String, TextureHandle>,
    settings: Settings,
    hub: SensorHub,
    director: Director,
    decision: Decision,
    simulated: State,
    simulation: bool,
    forced: Option<String>,
    start: Instant,
    last_frame: Instant,
    animation: String,
    elapsed: f64,
    frame: u32,
    paused: bool,
    speed: f64,
    lab_open: bool,
    tab: usize,
    background: usize,
    error: Option<String>,
    dirty: Option<Instant>,
    initialized: bool,
    action_rx: mpsc::Receiver<Action>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    _tray: Option<tray_icon::TrayIcon>,
    _watcher: Option<notify::RecommendedWatcher>,
    reload_rx: mpsc::Receiver<()>,
    reload_at: Option<Instant>,
    city_query: String,
    cities: Vec<weather::City>,
    city_rx: Option<mpsc::Receiver<Result<Vec<weather::City>, String>>>,
    pack_input: String,
}
impl Companion {
    fn new(
        cc: &eframe::CreationContext<'_>,
        pack: CharacterPack,
        mut settings: Settings,
        lab: bool,
    ) -> Self {
        let ctx = &cc.egui_ctx;
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = PLUM;
        visuals.window_fill = PLUM;
        visuals.override_text_color = Some(Color32::from_rgb(37, 49, 61));
        visuals.selection.bg_fill = Color32::from_rgb(149, 207, 199);
        visuals.widgets.active.bg_fill = Color32::from_rgb(149, 207, 199);
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(10., 10.);
            s.spacing.button_padding = Vec2::new(12., 7.);
        });
        settings.character = Some(pack.root.clone());
        let textures = Self::load_textures(ctx, &pack);
        let wake = ctx.clone();
        let hub = SensorHub::start(settings.clone(), move || wake.request_repaint());
        let (action_tx, action_rx) = mpsc::channel();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let (tray, tray_error) = match create_tray(ctx.clone(), &pack, action_tx) {
            Ok(tray) => (Some(tray), None),
            Err(e) => (
                None,
                Some(format!(
                    "Menu-bar icon unavailable: {e}. Click-through has been disabled."
                )),
            ),
        };
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let tray_error: Option<String> = {
            let _ = action_tx;
            Some("Linux desktop is experimental; use right-click controls. Click-through is disabled without a tray.".into())
        };
        if tray_error.is_some() {
            settings.click_through = false;
        }
        let (watcher, reload_rx) = Self::watch(ctx, &pack);
        let mut director = Director::default();
        let decision = director.update(&State::default(), 0);
        let lab_open = lab || tray_error.is_some();
        settings.first_run = false;
        Self {
            smoke: false,
            activity: None,
            hovered: false,
            dance_party: false,
            bubble: None,
            speech_visible: false,
            dialogue_index: 0,

            pack_input: pack.root.display().to_string(),
            pack,
            textures,
            settings,
            hub,
            director,
            decision,
            simulated: State::demo(),
            simulation: false,
            forced: None,
            start: Instant::now(),
            last_frame: Instant::now(),
            animation: "idle".into(),
            elapsed: 0.,
            frame: 0,
            paused: false,
            speed: 1.,
            lab_open,
            tab: 0,
            background: 0,
            error: tray_error,
            dirty: Some(Instant::now()),
            initialized: false,
            action_rx,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            _tray: tray,
            _watcher: watcher,
            reload_rx,
            reload_at: None,
            city_query: String::new(),
            cities: Vec::new(),
            city_rx: None,
        }
    }
    fn load_textures(ctx: &egui::Context, pack: &CharacterPack) -> BTreeMap<String, TextureHandle> {
        pack.sheets
            .iter()
            .map(|(name, s)| {
                (
                    name.clone(),
                    ctx.load_texture(
                        name,
                        egui::ColorImage::from_rgba_unmultiplied(
                            [s.width as usize, s.height as usize],
                            &s.rgba,
                        ),
                        egui::TextureOptions::NEAREST,
                    ),
                )
            })
            .collect()
    }
    fn watch(
        ctx: &egui::Context,
        pack: &CharacterPack,
    ) -> (Option<notify::RecommendedWatcher>, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::channel();
        let wake = ctx.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok_and(|e| !matches!(e.kind, notify::EventKind::Access(_))) {
                    let _ = tx.send(());
                    wake.request_repaint();
                }
            })
            .ok();
        if let Some(w) = &mut watcher {
            if w.watch(&pack.root, notify::RecursiveMode::Recursive)
                .is_err()
            {
                watcher = None;
            }
        }
        (watcher, rx)
    }
    fn state(&self) -> State {
        if self.simulation {
            self.simulated.clone()
        } else {
            self.hub.snapshot().state
        }
    }
    fn mark_dirty(&mut self) {
        self.dirty = Some(Instant::now());
        self.hub.configure(self.settings.clone());
    }
    fn tray_available(&self) -> bool {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            self._tray.is_some()
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }
    fn action(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::TerminalOnly => match crate::launch::open_terminal(&[]) {
                Ok(()) => ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close),
                Err(error) => self.error = Some(format!("Could not open terminal mode: {error}")),
            },
            Action::Lab => self.lab_open = true,
            Action::Pause => self.paused = !self.paused,
            Action::ClickThrough if self.tray_available() => {
                self.settings.click_through = !self.settings.click_through;
                ctx.send_viewport_cmd_to(
                    ViewportId::ROOT,
                    ViewportCommand::MousePassthrough(self.settings.click_through),
                );
                self.mark_dirty();
            }
            Action::ClickThrough => {}
            Action::Reset => {
                let size = ctx
                    .input(|i| i.viewport().monitor_size)
                    .unwrap_or(Vec2::new(1280., 800.));
                let [w, h] = self.pack.manifest.sprite_size;
                let position = Pos2::new(
                    (size.x - w as f32 * self.settings.scale as f32 - 48.).max(0.),
                    (size.y - h as f32 * self.settings.scale as f32 - 100.).max(0.),
                );
                ctx.send_viewport_cmd_to(
                    ViewportId::ROOT,
                    ViewportCommand::OuterPosition(position),
                );
                self.settings.position = Some([position.x, position.y]);
                self.mark_dirty();
            }
            Action::Scale(scale) => {
                self.settings.scale = scale;
                let [w, h] = self.pack.manifest.sprite_size;
                ctx.send_viewport_cmd_to(
                    ViewportId::ROOT,
                    ViewportCommand::InnerSize({
                        let body = Vec2::new(w as f32, h as f32) * scale as f32;
                        if self.speech_visible {
                            Vec2::new(body.x.max(180.), body.y + 54.)
                        } else {
                            body
                        }
                    }),
                );
                self.mark_dirty();
            }
            Action::Power(power) => {
                self.settings.power = power;
                self.mark_dirty();
            }
            Action::Quit => ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close),
        }
        ctx.request_repaint();
    }
    fn reload(&mut self, ctx: &egui::Context, path: std::path::PathBuf) {
        match CharacterPack::load(path) {
            Ok(pack) => {
                self.textures = Self::load_textures(ctx, &pack);
                (self._watcher, self.reload_rx) = Self::watch(ctx, &pack);
                self.settings.character = Some(pack.root.clone());
                self.pack_input = pack.root.display().to_string();
                self.pack = pack;
                self.animation.clear();
                self.forced = None;
                self.error = None;
                self.action(ctx, Action::Scale(self.settings.scale));
            }
            Err(e) => {
                self.error = Some(format!(
                    "Could not load pack: {e}. Keeping the previous character."
                ))
            }
        }
    }
    fn talk(&mut self, mood: &str) {
        let text = crate::dialogue::line(mood, self.dialogue_index);
        self.dialogue_index = self.dialogue_index.wrapping_add(1);
        self.bubble = Some((text.into(), Instant::now()));
    }

    fn resize_for_speech(&mut self, ctx: &egui::Context, visible: bool) {
        if self.speech_visible == visible {
            return;
        }
        let [w, h] = self.pack.manifest.sprite_size;
        let body = Vec2::new(w as f32, h as f32) * self.settings.scale as f32;
        let expanded = Vec2::new(body.x.max(180.), body.y + 54.);
        let offset = Vec2::new((expanded.x - body.x) / 2., 54.);
        if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
            // Keep the mascot's feet in the same place as the bubble appears or disappears.
            let position = if visible {
                rect.min - offset
            } else {
                rect.min + offset
            };
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(position));
        }
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(if visible {
            expanded
        } else {
            body
        }));
        self.speech_visible = visible;
        ctx.request_repaint();
    }

    fn draw_character(&self, ui: &mut egui::Ui, rect: Rect, overlays: bool) {
        let (name, animation) = self.pack.resolve(&self.animation);
        let frame = self.frame.min(animation.frames - 1);
        let uv = Rect::from_min_max(
            Pos2::new(frame as f32 / animation.frames as f32, 0.),
            Pos2::new((frame + 1) as f32 / animation.frames as f32, 1.),
        );
        ui.painter()
            .image(self.textures[name].id(), rect, uv, Color32::WHITE);
        if !overlays || !self.pack.manifest.kind.is_empty() {
            return;
        }
        let sx = rect.width() / self.pack.manifest.sprite_size[0] as f32;
        let sy = rect.height() / self.pack.manifest.sprite_size[1] as f32;
        let pixel = |x: f32, y: f32, w: f32, h: f32, color: Color32| {
            ui.painter().rect_filled(
                Rect::from_min_size(
                    rect.min + Vec2::new(x * sx, y * sy),
                    Vec2::new(w * sx, h * sy),
                ),
                0.,
                color,
            );
        };
        if self.decision.headphones {
            pixel(7., 8., 2., 7., Color32::from_rgb(124, 180, 205));
            pixel(24., 8., 2., 7., Color32::from_rgb(124, 180, 205));
            pixel(9., 6., 15., 1., Color32::from_rgb(119, 132, 139));
        }
        if self.decision.rain {
            for (x, y) in [(3., 4.), (28., 12.), (5., 22.)] {
                pixel(
                    x,
                    y + self.frame as f32,
                    1.,
                    2.,
                    Color32::from_rgb(124, 180, 205),
                );
            }
        }
        if self.decision.heat && !name.starts_with("cpu") {
            pixel(27., 9., 1., 3., Color32::from_rgb(238, 128, 83));
        }
    }
    fn lab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(PLUM).inner_margin(24))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Character Lab").size(28.).strong());
                        ui.label(
                            RichText::new("A little attitude. A corner of your desktop.")
                                .color(MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Back to desktop").clicked() {
                            self.lab_open = false;
                        }
                    });
                });
                ui.add_space(18.);
                ui.columns(2, |columns| {
                    let left = &mut columns[0];
                    let width = left.available_width();
                    let (area, _) =
                        left.allocate_exact_size(Vec2::new(width, 280.), Sense::hover());
                    let bg = [
                        PAPER,
                        Color32::from_rgb(23, 28, 34),
                        Color32::from_rgb(117, 134, 119),
                    ][self.background];
                    left.painter().rect_filled(area, 12., bg);
                    for x in (0..width as usize).step_by(16) {
                        for y in (0..280).step_by(16) {
                            if (x / 16 + y / 16) % 2 == 0 {
                                left.painter().rect_filled(
                                    Rect::from_min_size(
                                        area.min + Vec2::new(x as f32, y as f32),
                                        Vec2::splat(16.),
                                    )
                                    .intersect(area),
                                    0.,
                                    Color32::from_black_alpha(6),
                                );
                            }
                        }
                    }
                    {
                        let size = Vec2::splat(((width - 32.) / 64.).floor().min(4.) * 64.);
                        self.draw_character(
                            left,
                            Rect::from_center_size(area.center(), size),
                            true,
                        );
                    }
                    left.add_space(10.);
                    left.label(RichText::new(&self.pack.manifest.name).size(23.).strong());
                    left.label(
                        RichText::new(
                            self.pack
                                .manifest
                                .captions
                                .get(&self.animation)
                                .map(String::as_str)
                                .unwrap_or(self.decision.reason),
                        )
                        .color(AMBER),
                    );
                    left.add_space(8.);
                    left.horizontal(|ui| {
                        if ui
                            .button(if self.paused { "Resume" } else { "Pause" })
                            .clicked()
                        {
                            self.paused = !self.paused;
                        }
                        if ui.button("Step frame").clicked() {
                            self.paused = true;
                            let (_, a) = self.pack.resolve(&self.animation);
                            self.frame = (self.frame + 1) % a.frames;
                            self.elapsed =
                                a.durations_ms[..self.frame as usize].iter().sum::<u64>() as f64;
                        }
                        if ui.button("Backdrop").clicked() {
                            self.background = (self.background + 1) % 3;
                        }
                    });
                    left.add(egui::Slider::new(&mut self.speed, 0.25..=2.).text("Playback speed"));
                    left.separator();
                    left.label(
                        RichText::new(if self.simulation {
                            "Simulated signals"
                        } else {
                            "Live device signals"
                        })
                        .color(AMBER),
                    );
                    left.label(format!(
                        "Animation: {}   Frame: {}",
                        self.animation,
                        self.frame + 1
                    ));
                    left.label(format!(
                        "Intensity: {:?}   Priority: {}",
                        self.decision.intensity, self.decision.priority
                    ));
                    left.label(format!(
                        "Power: {:?}",
                        self.settings.power.effective(&self.state())
                    ));
                    egui::CollapsingHeader::new("Director details").show(left, |ui| {
                        ui.label(format!("Queue: {:?}", self.director.queued()));
                        let cooldowns = self
                            .director
                            .cooldowns(self.start.elapsed().as_millis() as u64);
                        if cooldowns.is_empty() {
                            ui.label("No active cooldowns");
                        }
                        for (name, seconds) in cooldowns {
                            ui.label(format!("{name}: {seconds}s"));
                        }
                        ui.label("CPU: enter >80% for 5s; leave <60% for 10s.");
                        ui.label("Memory: enter >85% for 5s; leave <80% for 10s.");
                    });

                    let right = &mut columns[1];
                    right.horizontal(|ui| {
                        ui.selectable_value(&mut self.tab, 0, "Reactions");
                        ui.selectable_value(&mut self.tab, 1, "Settings");
                        ui.selectable_value(&mut self.tab, 2, "Privacy");
                        ui.selectable_value(&mut self.tab, 3, "Health");
                    });
                    right.separator();
                    egui::ScrollArea::vertical()
                        .max_height(490.)
                        .show(right, |ui| match self.tab {
                            0 => self.reactions(ui),
                            1 => self.settings_ui(ui),
                            3 => self.health(ui),
                            _ => self.privacy(ui),
                        });
                });
                if let Some(error) = &self.error {
                    ui.separator();
                    ui.label(RichText::new(error).color(Color32::from_rgb(255, 169, 136)));
                }
            });
    }
    fn reactions(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.dance_party, "Dance party");
        ui.label("Glitch also dances automatically when output audio is active. Manual dance works without device readings.");
        ui.label("Say hello or give Glitch a little activity.");
        ui.horizontal_wrapped(|ui| {
            for (name, label) in [
                ("wave", "Wave"),
                ("walk", "Stretch"),
                ("celebrate", "Happy hop"),
                ("sleep", "Tiny nap"),
                ("curious", "Be curious"),
            ] {
                if ui.button(label).clicked() {
                    self.forced = None;
                    self.activity = Some((name.into(), Instant::now()));
                    self.talk(name);
                    self.elapsed = 0.;
                    ui.ctx().request_repaint();
                }
            }
        });
        ui.label("Click Glitch to wave. Double-click for these controls. Drag to move.");
        ui.separator();
        if ui
            .checkbox(&mut self.simulation, "Simulate device state")
            .changed()
        {
            self.director = Director::default();
            self.forced = None;
        }
        ui.label(
            RichText::new("Simulation changes this character only.")
                .small()
                .color(MUTED),
        );
        if self.simulation {
            let s = &mut self.simulated;
            let battery = s.battery.get_or_insert(Battery {
                level: 72.,
                charging: false,
            });
            ui.add(egui::Slider::new(&mut battery.level, 0.0..=100.).text("Battery %"));
            ui.checkbox(&mut battery.charging, "Charging");
            ui.add(egui::Slider::new(s.cpu.get_or_insert(12.), 0.0..=100.).text("CPU %"));
            ui.add(egui::Slider::new(s.memory.get_or_insert(40.), 0.0..=100.).text("Memory %"));
            ui.checkbox(s.connected.get_or_insert(true), "Network connected");
            ui.add(egui::Slider::new(&mut s.hour, 0..=23).text("Hour"));
            egui::ComboBox::from_id_salt("day")
                .selected_text(
                    [
                        "Monday",
                        "Tuesday",
                        "Wednesday",
                        "Thursday",
                        "Friday",
                        "Saturday",
                        "Sunday",
                    ][s.day as usize],
                )
                .show_ui(ui, |ui| {
                    for (i, day) in [
                        "Monday",
                        "Tuesday",
                        "Wednesday",
                        "Thursday",
                        "Friday",
                        "Saturday",
                        "Sunday",
                    ]
                    .iter()
                    .enumerate()
                    {
                        ui.selectable_value(&mut s.day, i as u8, *day);
                    }
                });
            egui::ComboBox::from_id_salt("weather")
                .selected_text(format!("Weather: {:?}", s.weather))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.weather, None, "No weather");
                    for w in [
                        Weather::Clear,
                        Weather::Cloudy,
                        Weather::Rain,
                        Weather::Snow,
                        Weather::Thunder,
                        Weather::Hot,
                        Weather::Cold,
                    ] {
                        ui.selectable_value(&mut s.weather, Some(w), format!("{w:?}"));
                    }
                });
            ui.add(egui::Slider::new(s.volume.get_or_insert(35.), 0.0..=100.).text("Volume %"));
            ui.checkbox(s.headphones.get_or_insert(false), "Headphones connected");
            ui.checkbox(s.audio_playing.get_or_insert(false), "Audio playing");
            egui::ComboBox::from_id_salt("bluetooth")
                .selected_text(format!("Bluetooth: {:?}", s.bluetooth))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.bluetooth, None, "Disconnected / unavailable");
                    for b in [
                        BluetoothCategory::Audio,
                        BluetoothCategory::Controller,
                        BluetoothCategory::Accessory,
                        BluetoothCategory::Generic,
                    ] {
                        ui.selectable_value(&mut s.bluetooth, Some(b), format!("{b:?}"));
                    }
                });
            if ui.button("Reset simulation").clicked() {
                self.simulated = State::demo();
                self.director = Director::default();
            }
        } else {
            let snapshot = self.hub.snapshot();
            let s = snapshot.state;
            let percent = |v: Option<f32>| {
                v.map(|v| format!("{v:.0}%"))
                    .unwrap_or_else(|| "Unavailable".into())
            };
            egui::Grid::new("readings")
                .spacing([16., 12.])
                .show(ui, |ui| {
                    for (label, value) in [
                        (
                            "Battery",
                            s.battery
                                .map(|b| {
                                    format!(
                                        "{:.0}%{}",
                                        b.level,
                                        if b.charging { " · charging" } else { "" }
                                    )
                                })
                                .unwrap_or_else(|| "No reading".into()),
                        ),
                        ("CPU", percent(s.cpu)),
                        ("Memory", percent(s.memory)),
                        (
                            "Network",
                            match s.connected {
                                Some(true) => "Local connection",
                                Some(false) => "Offline",
                                None => "Unavailable",
                            }
                            .into(),
                        ),
                        ("Volume", percent(s.volume)),
                        (
                            "Headphones",
                            match s.headphones {
                                Some(true) => "Connected",
                                Some(false) => "Not detected",
                                None => "Unavailable",
                            }
                            .into(),
                        ),
                        (
                            "Bluetooth",
                            s.bluetooth
                                .map(|b| format!("{b:?}"))
                                .unwrap_or_else(|| "None / unavailable".into()),
                        ),
                        ("Time", format!("{:02}:00", s.hour)),
                    ] {
                        ui.label(RichText::new(label).color(MUTED));
                        ui.label(value);
                        ui.end_row();
                    }
                });
            ui.add_space(8.);
            ui.label(RichText::new(snapshot.note).small().color(MUTED));
        }
        ui.separator();
        egui::ComboBox::from_id_salt("animation")
            .selected_text(self.forced.as_deref().unwrap_or("Director chooses"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.forced, None, "Director chooses");
                for name in self.pack.manifest.animations.keys() {
                    ui.selectable_value(&mut self.forced, Some(name.clone()), name);
                }
            });
    }
    fn health(&mut self, ui: &mut egui::Ui) {
        let state = self.state();
        ui.heading("How your computer is doing");
        ui.label("Live usage, with warnings for sustained load.");
        for (label, reading, busy, threshold) in [
            ("CPU", state.cpu, self.decision.cpu_busy, 80.),
            ("RAM", state.memory, self.decision.memory_busy, 85.),
        ] {
            ui.add_space(12.);
            ui.label(RichText::new(label).strong());
            if let Some(value) = reading {
                ui.add(
                    egui::ProgressBar::new(value / 100.)
                        .text(format!("{value:.0}% used"))
                        .fill(if busy {
                            Color32::from_rgb(219, 135, 77)
                        } else {
                            AMBER
                        }),
                );
                ui.label(if busy {
                    "Sustained high usage"
                } else if value > threshold {
                    "Brief spike — watching before warning"
                } else {
                    "Comfortable usage"
                });
            } else {
                ui.label("Waiting for a reading");
            }
        }
        ui.add_space(16.);
        if self.decision.memory_busy {
            ui.label("RAM: close apps or browser tabs you are not using. Save work before closing anything.");
        }
        if self.decision.cpu_busy {
            ui.label("CPU: let heavy tasks finish, or pause work you don't need right now. High use can be normal during demanding tasks.");
        }
        if !self.decision.cpu_busy && !self.decision.memory_busy {
            ui.label("No sustained CPU or RAM warning right now.");
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if ui.button("See which apps are busy").clicked() {
            #[cfg(target_os = "macos")]
            let result = std::process::Command::new("/usr/bin/open")
                .args(["-a", "Activity Monitor"])
                .spawn();
            #[cfg(target_os = "windows")]
            let result = std::process::Command::new("taskmgr.exe").spawn();
            match result {
                Ok(mut child) => {
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                }
                Err(error) => {
                    self.error = Some(format!("Could not open the system monitor: {error}"));
                }
            }
        }
        ui.separator();
        ui.label("CPU warns above 80% for 5 seconds; RAM above 85% for 5 seconds. Warnings clear after 10 seconds below 60% CPU or 80% RAM.");
        ui.label("RAM shows used memory, not a diagnosis of memory pressure or a list of apps. Glitch does not close apps or modify your computer.");
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Companion mode");
        ui.label("Desktop: floats over your other apps. Terminal only: lives inside a terminal, with no desktop mascot.");
        if ui.button("Switch to terminal only").clicked() {
            self.action(ui.ctx(), Action::TerminalOnly);
        }
        ui.label("Dialogue appears only when you click or choose an activity. Health warnings use small badges.");
        ui.separator();
        ui.label(RichText::new("Your companion").size(18.).strong());
        let ctx = ui.ctx().clone();
        let parent = self.pack.root.parent().unwrap().to_path_buf();
        ui.horizontal(|ui| {
            for (id, label) in [
                ("glitch", "Glitch"),
                ("mochi", "Mochi · puppy"),
                ("pip", "Pip"),
            ] {
                if parent.join(id).join("character.json").exists()
                    && ui
                        .selectable_label(self.pack.manifest.id == id, label)
                        .clicked()
                {
                    self.reload(&ctx, parent.join(id));
                }
            }
        });
        ui.separator();
        ui.label(RichText::new("Desktop").size(18.).strong());
        let ctx = ui.ctx().clone();
        let mut scale = self.settings.scale;
        if ui
            .add(egui::Slider::new(&mut scale, 2..=8).text("Pixel scale"))
            .changed()
        {
            self.action(&ctx, Action::Scale(scale));
        }
        let mut through = self.settings.click_through;
        if ui
            .add_enabled(
                self.tray_available(),
                egui::Checkbox::new(&mut through, "Click through the companion"),
            )
            .changed()
        {
            self.action(&ctx, Action::ClickThrough);
        }
        ui.label(
            RichText::new("Use the menu-bar / tray icon to turn interaction back on.")
                .small()
                .color(MUTED),
        );
        if ui.button("Reset desktop position").clicked() {
            self.action(&ctx, Action::Reset);
        }
        let before = self.settings.power;
        egui::ComboBox::from_id_salt("power")
            .selected_text(format!("Power: {:?}", before))
            .show_ui(ui, |ui| {
                for mode in [
                    PowerMode::Auto,
                    PowerMode::Normal,
                    PowerMode::Saver,
                    PowerMode::Ultra,
                ] {
                    ui.selectable_value(&mut self.settings.power, mode, format!("{mode:?}"));
                }
            });
        if before != self.settings.power {
            self.mark_dirty();
        }
        if ui
            .checkbox(
                &mut self.settings.sensors_enabled,
                "Read local device state",
            )
            .changed()
        {
            self.mark_dirty();
        }
        ui.separator();
        ui.label(RichText::new("Character pack").size(18.).strong());
        ui.text_edit_singleline(&mut self.pack_input);
        ui.horizontal(|ui| {
            if ui.button("Load folder").clicked() {
                self.reload(&ctx, std::path::PathBuf::from(&self.pack_input));
            }
            if ui.button("Reload assets").clicked() {
                self.reload(&ctx, self.pack.root.clone());
            }
        });
        ui.label(
            RichText::new(if self._watcher.is_some() {
                "Asset hot reload is watching this folder."
            } else {
                "Automatic reload unavailable. Use Reload assets."
            })
            .small()
            .color(MUTED),
        );
        ui.separator();
        ui.label(RichText::new("Optional weather").size(18.).strong());
        ui.label("Choose a city. No GPS or precise location.");
        ui.label(RichText::new(self.hub.snapshot().weather_status).color(MUTED));
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.city_query)
                    .hint_text("City name")
                    .desired_width(170.),
            );
            if ui
                .add_enabled(self.city_rx.is_none(), egui::Button::new("Search"))
                .clicked()
            {
                let query = self.city_query.clone();
                let (tx, rx) = mpsc::channel();
                let wake = ctx.clone();
                self.city_rx = Some(rx);
                std::thread::spawn(move || {
                    let _ = tx.send(weather::search(&query).map_err(|e| e.to_string()));
                    wake.request_repaint();
                });
            }
        });
        ui.label(RichText::new("Search sends your city query to Open-Meteo. Selecting a city enables a forecast request every 30 minutes.").small().color(MUTED));
        for city in self.cities.clone() {
            if ui
                .button(format!("{}, {}", city.name, city.country))
                .clicked()
            {
                self.settings.weather_city = Some(city);
                self.cities.clear();
                self.mark_dirty();
            }
        }
        if self.settings.weather_city.is_some() && ui.button("Turn weather off").clicked() {
            self.settings.weather_city = None;
            self.mark_dirty();
        }
        ui.hyperlink_to("Weather data by Open-Meteo", "https://open-meteo.com/");
    }
    fn privacy(&self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("A roommate, not a watcher.")
                .size(22.)
                .strong(),
        );
        ui.add_space(12.);
        ui.label("Local readings");
        ui.label("Audio output activity, battery and charging, aggregate CPU and memory usage, local network interface state, clock, output volume, supported headphone routes, and generic connected Bluetooth category.");
        ui.add_space(12.);
        ui.label("Never inspected");
        ui.label("Screen contents, screenshots, keystrokes, clipboard, messages, browser history, microphone, camera, audio streams, network traffic, or your documents.");
        ui.add_space(12.);
        ui.label("What is saved");
        ui.label("Local preferences: position, scale, character folder, click-through, power mode, and an optional weather city. Device readings are not logged.");
        ui.add_space(12.);
        ui.label("Network access");
        ui.label("Only optional Open-Meteo city search and weather. No account, cloud sync, analytics, or AI service.");
        ui.add_space(12.);
        ui.label("Unavailable readings stay unavailable. Bluetooth support covers connected paired classic devices; BLE-only accessories and some headphone routes are not reported.");
        ui.add_space(12.);
        ui.label("Packs contain only validated PNG images and JSON animation data. No scripts or executables.");
    }
}
impl eframe::App for Companion {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0., 0., 0., 0.]
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.smoke {
            let limit = Duration::from_secs(20);
            if self.start.elapsed() >= limit {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else {
                ctx.request_repaint_after(limit.saturating_sub(self.start.elapsed()));
            }
        }
        if !self.initialized {
            self.initialized = true;
            if self.settings.position.is_none() {
                self.action(ctx, Action::Reset);
            }
            ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(
                self.settings.click_through,
            ));
        }
        while let Ok(action) = self.action_rx.try_recv() {
            self.action(ctx, action);
        }
        while self.reload_rx.try_recv().is_ok() {
            self.reload_at = Some(Instant::now());
        }
        if self
            .reload_at
            .is_some_and(|t| t.elapsed() > Duration::from_millis(250))
        {
            self.reload_at = None;
            self.reload(ctx, self.pack.root.clone());
        }
        if self.reload_at.is_some() {
            ctx.request_repaint_after(Duration::from_millis(260));
        }
        if let Some(result) = self.city_rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.city_rx = None;
            match result {
                Ok(cities) => {
                    self.error = if cities.is_empty() {
                        Some("No cities found. Try a nearby city or a different spelling.".into())
                    } else {
                        None
                    };
                    self.cities = cities;
                }
                Err(e) => self.error = Some(format!("City search failed: {e}")),
            }
        }
        let state = self.state();
        let now = self.start.elapsed().as_millis() as u64;
        let delta = self.last_frame.elapsed().as_secs_f64() * 1000.;
        self.last_frame = Instant::now();
        if !self.paused {
            self.decision = self.director.update(&state, now);
            if self.activity.as_ref().is_some_and(|(name, t)| {
                let (_, animation) = self.pack.resolve(name);
                let duration: u64 = animation.durations_ms.iter().sum();
                t.elapsed() > Duration::from_millis(duration.max(1200))
            }) {
                self.activity = None;
            }
            let requested = self
                .forced
                .as_deref()
                .or_else(|| self.activity.as_ref().map(|(name, _)| name.as_str()))
                .unwrap_or(if self.dance_party && self.decision.priority < 60 {
                    "dance"
                } else {
                    self.decision.animation
                });
            let (resolved, _) = self.pack.resolve(requested);
            if self.animation != resolved {
                self.animation = resolved.into();
                self.elapsed = 0.;
            } else {
                self.elapsed += delta * self.speed;
            }
            let (_, animation) = self.pack.resolve(&self.animation);
            let (frame, delay) = animation.frame_at(self.elapsed as u64);
            self.frame = frame;
            let floor = self.settings.power.effective(&state).frame_floor_ms();
            ctx.request_repaint_after(Duration::from_millis(
                ((delay as f64 / self.speed) as u64).max(floor),
            ));
        }
        if self
            .bubble
            .as_ref()
            .is_some_and(|(_, created)| created.elapsed() >= Duration::from_secs(5))
        {
            self.bubble = None;
        }
        self.resize_for_speech(ctx, self.bubble.is_some());
        if let Some((_, created)) = &self.bubble {
            ctx.request_repaint_after(Duration::from_secs(5).saturating_sub(created.elapsed()));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let area = ui.max_rect();
                let [w, h] = self.pack.manifest.sprite_size;
                let size = Vec2::new(w as f32, h as f32) * self.settings.scale as f32;
                let rect = Rect::from_min_size(
                    Pos2::new(area.center().x - size.x / 2., area.bottom() - size.y),
                    size,
                );
                let response =
                    ui.interact(rect, ui.id().with("companion"), Sense::click_and_drag());
                let hovered = response.hovered() && !response.dragged();
                if hovered && !self.hovered && !self.paused {
                    self.activity = Some(("curious".into(), Instant::now()));
                    self.elapsed = 0.;
                    ctx.request_repaint();
                }
                self.hovered = hovered;
                if hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    ui.painter().circle_filled(
                        rect.center() + Vec2::new(0., size.y * 0.15),
                        size.x * 0.26,
                        Color32::from_rgba_unmultiplied(91, 189, 171, 30),
                    );
                    for offset in [
                        Vec2::new(-size.x * 0.3, -size.y * 0.1),
                        Vec2::new(size.x * 0.3, -size.y * 0.22),
                    ] {
                        let center = rect.center() + offset;
                        ui.painter().rect_filled(
                            Rect::from_center_size(center, Vec2::new(8., 2.)),
                            0.,
                            Color32::from_rgb(245, 189, 74),
                        );
                        ui.painter().rect_filled(
                            Rect::from_center_size(center, Vec2::new(2., 8.)),
                            0.,
                            Color32::from_rgb(245, 189, 74),
                        );
                    }
                }
                self.draw_character(
                    ui,
                    if hovered {
                        rect.shrink(2.).translate(Vec2::new(0., -3.))
                    } else {
                        rect
                    },
                    true,
                );
                let mut badge_y = 5.;
                for (label, reading, busy) in [
                    ("CPU", state.cpu, self.decision.cpu_busy),
                    ("RAM", state.memory, self.decision.memory_busy),
                ] {
                    if busy {
                        let badge = Rect::from_min_size(
                            rect.min + Vec2::new(2., badge_y),
                            Vec2::new(61., 16.),
                        );
                        ui.painter()
                            .rect_filled(badge, 4., Color32::from_rgb(255, 237, 211));
                        let text = reading
                            .map(|v| format!("{label} {v:.0}%"))
                            .unwrap_or_else(|| format!("{label} busy"));
                        ui.painter().text(
                            badge.center(),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::proportional(10.),
                            Color32::from_rgb(116, 69, 39),
                        );
                        if ui
                            .interact(badge, ui.id().with(label), Sense::click())
                            .clicked()
                        {
                            self.tab = 3;
                            self.lab_open = true;
                        }
                        badge_y += 18.;
                    }
                }
                if let Some(battery) = state.battery.filter(|b| b.charging || b.level < 20.) {
                    let badge = Rect::from_min_size(
                        rect.min + Vec2::new(size.x * 0.7, 6.),
                        Vec2::new(14., 7.),
                    );
                    let color = if battery.charging {
                        Color32::from_rgb(51, 158, 133)
                    } else {
                        Color32::from_rgb(221, 119, 80)
                    };
                    ui.painter().rect(
                        badge,
                        1.,
                        Color32::from_rgb(255, 251, 240),
                        egui::Stroke::new(1_f32, color),
                        egui::StrokeKind::Inside,
                    );
                    let fill = Rect::from_min_size(
                        badge.min + Vec2::splat(2.),
                        Vec2::new(10. * battery.level.clamp(0., 100.) / 100., 3.),
                    );
                    ui.painter().rect_filled(fill, 0., color);
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            badge.right_top() + Vec2::new(0., 2.),
                            Vec2::new(2., 3.),
                        ),
                        0.,
                        color,
                    );
                }
                if let Some((text, _)) = &self.bubble {
                    let bubble_rect = Rect::from_min_size(
                        Pos2::new(area.left() + 5., area.top() + 4.),
                        Vec2::new(area.width() - 10., 40.),
                    );
                    ui.painter().rect(
                        bubble_rect,
                        12.,
                        Color32::from_rgb(255, 251, 240),
                        egui::Stroke::new(1_f32, Color32::from_rgb(203, 219, 211)),
                        egui::StrokeKind::Inside,
                    );
                    let tail = bubble_rect.center_bottom();
                    ui.painter().add(egui::Shape::convex_polygon(
                        vec![
                            tail + Vec2::new(-6., -1.),
                            tail + Vec2::new(6., -1.),
                            tail + Vec2::new(0., 8.),
                        ],
                        Color32::from_rgb(255, 251, 240),
                        egui::Stroke::NONE,
                    ));
                    let galley = ui.painter().layout(
                        text.clone(),
                        egui::FontId::proportional(12.),
                        Color32::from_rgb(52, 66, 69),
                        bubble_rect.width() - 18.,
                    );
                    ui.painter().galley(
                        bubble_rect.center() - galley.size() / 2.,
                        galley,
                        Color32::from_rgb(52, 66, 69),
                    );
                    if ui
                        .interact(bubble_rect, ui.id().with("speech"), Sense::click())
                        .clicked()
                    {
                        self.bubble = None;
                    }
                }
                if response.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if response.clicked() && !response.double_clicked() {
                    self.activity = Some(("wave".into(), Instant::now()));
                    self.talk("click");
                    self.elapsed = 0.;
                    ctx.request_repaint();
                }
                if response.double_clicked() || response.secondary_clicked() {
                    self.lab_open = true;
                }
            });
        if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
            let [w, _] = self.pack.manifest.sprite_size;
            let body_width = w as f32 * self.settings.scale as f32;
            let offset = if self.speech_visible {
                Vec2::new((body_width.max(180.) - body_width) / 2., 54.)
            } else {
                Vec2::ZERO
            };
            let pos = [rect.min.x + offset.x, rect.min.y + offset.y];
            if self.settings.position != Some(pos) {
                self.settings.position = Some(pos);
                self.dirty = Some(Instant::now());
            }
        }
        if self
            .dirty
            .is_some_and(|t| t.elapsed() >= Duration::from_millis(800))
        {
            self.dirty = None;
            if let Err(e) = self.settings.save() {
                self.error = Some(format!("Could not save preferences: {e}"));
            }
        }
        if self.dirty.is_some() {
            ctx.request_repaint_after(Duration::from_millis(850));
        }
        if self.lab_open {
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of("character-lab"),
                egui::ViewportBuilder::default()
                    .with_title("Scopobot · Character Lab")
                    .with_inner_size([850., 680.])
                    .with_min_inner_size([760., 640.]),
                |ctx, _class| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        self.lab_open = false;
                    }
                    self.lab(ctx);
                },
            );
        }
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        let _ = self.settings.save();
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn create_tray(
    ctx: egui::Context,
    pack: &CharacterPack,
    tx: mpsc::Sender<Action>,
) -> anyhow::Result<tray_icon::TrayIcon> {
    use tray_icon::{
        menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
        Icon, TrayIconBuilder,
    };
    let menu = Menu::new();
    let mut actions = std::collections::HashMap::new();
    for (label, action) in [
        ("Open Character Lab", Action::Lab),
        ("Pause / resume", Action::Pause),
        ("Toggle click-through", Action::ClickThrough),
        ("Reset position", Action::Reset),
    ] {
        let item = MenuItem::new(label, true, None);
        actions.insert(item.id().clone(), action);
        menu.append(&item)?;
    }
    let terminal = MenuItem::new("Switch to terminal only", true, None);
    actions.insert(terminal.id().clone(), Action::TerminalOnly);
    menu.append(&terminal)?;
    let scales = Submenu::new("Character size", true);
    for scale in [2, 3, 4, 5, 6, 8] {
        let item = MenuItem::new(format!("{scale}×"), true, None);
        actions.insert(item.id().clone(), Action::Scale(scale));
        scales.append(&item)?;
    }
    menu.append(&scales)?;
    let powers = Submenu::new("Power mode", true);
    for mode in [
        PowerMode::Auto,
        PowerMode::Normal,
        PowerMode::Saver,
        PowerMode::Ultra,
    ] {
        let item = MenuItem::new(format!("{mode:?}"), true, None);
        actions.insert(item.id().clone(), Action::Power(mode));
        powers.append(&item)?;
    }
    menu.append(&powers)?;
    menu.append(&PredefinedMenuItem::separator())?;
    let quit = MenuItem::new("Quit Scopobot", true, None);
    actions.insert(quit.id().clone(), Action::Quit);
    menu.append(&quit)?;
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if let Some(action) = actions.get(&event.id) {
            let _ = tx.send(*action);
            ctx.request_repaint();
        }
    }));
    let s = &pack.sheets["idle"];
    let [w, h] = pack.manifest.sprite_size;
    let mut pixels = Vec::new();
    for y in 0..h {
        let start = (y * s.width * 4) as usize;
        pixels.extend_from_slice(&s.rgba[start..start + (w * 4) as usize]);
    }
    Ok(TrayIconBuilder::new()
        .with_tooltip("Scopobot — your tiny desktop roommate")
        .with_menu(Box::new(menu))
        .with_icon(Icon::from_rgba(pixels, w, h)?)
        .build()?)
}
