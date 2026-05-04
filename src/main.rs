use directories::ProjectDirs;
use eframe::egui::{
    self, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 560.0])
            .with_min_inner_size([360.0, 540.0])
            .with_title("Retro Pet"),
        ..Default::default()
    };

    eframe::run_native(
        "Retro Pet",
        options,
        Box::new(|cc| Ok(Box::new(RetroPetApp::new(cc)))),
    )
}

struct RetroPetApp {
    pet: Pet,
    last_tick: Instant,
    last_save: Instant,
    save_path: PathBuf,
    save_dirty: bool,
    message: String,
    message_until: Instant,
}

impl RetroPetApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(18, 20, 18);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(48, 58, 47);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 84, 62);
        visuals.widgets.active.bg_fill = Color32::from_rgb(111, 135, 88);
        cc.egui_ctx.set_visuals(visuals);

        let save_path = save_path();
        let pet = load_pet(&save_path);
        let loaded_existing_pet = pet.is_some();
        let now = Instant::now();

        Self {
            pet: pet.unwrap_or_default(),
            last_tick: Instant::now(),
            last_save: now,
            save_path,
            save_dirty: !loaded_existing_pet,
            message: if loaded_existing_pet {
                "Welcome back. It remembers you.".to_owned()
            } else {
                "A tiny friend blinks at you.".to_owned()
            },
            message_until: now + Duration::from_secs(4),
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        self.pet.update(elapsed);
        self.save_dirty = true;
        if now > self.message_until {
            self.message = self.pet.mood_line().to_owned();
            self.message_until = now + Duration::from_secs(3);
        }

        if now.duration_since(self.last_save) >= Duration::from_secs(5) {
            self.save();
        }
    }

    fn save(&mut self) {
        if !self.save_dirty {
            return;
        }

        match save_pet(&self.save_path, &self.pet) {
            Ok(()) => {
                self.save_dirty = false;
                self.last_save = Instant::now();
            }
            Err(error) => {
                self.message = format!("Save failed: {error}");
                self.message_until = Instant::now() + Duration::from_secs(4);
                self.last_save = Instant::now();
            }
        }
    }
}

impl eframe::App for RetroPetApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick();
        ctx.request_repaint_after(Duration::from_millis(100));

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
            ui.add_space(4.0);

            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("RETRO PET")
                        .font(FontId::monospace(28.0))
                        .color(Color32::from_rgb(198, 229, 159)),
                );
                ui.label(
                    RichText::new(format!(
                        "{}  |  Day {}  |  {} {}",
                        self.pet.name,
                        self.pet.day(),
                        self.pet.personality.label(),
                        self.pet.stage()
                    ))
                    .font(FontId::monospace(14.0))
                    .color(Color32::from_rgb(132, 165, 116)),
                );
            });

            draw_screen(ui, &self.pet, ctx.input(|input| input.time));

            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(&self.message)
                        .font(FontId::monospace(15.0))
                        .color(Color32::from_rgb(221, 239, 184)),
                );
            });

            ui.add_space(2.0);
            stat_bar(ui, "Food", self.pet.food, Color32::from_rgb(143, 203, 115));
            stat_bar(
                ui,
                "Happy",
                self.pet.happiness,
                Color32::from_rgb(238, 191, 96),
            );
            stat_bar(
                ui,
                "Energy",
                self.pet.energy,
                Color32::from_rgb(112, 169, 230),
            );
            stat_bar(
                ui,
                "Clean",
                self.pet.cleanliness,
                Color32::from_rgb(147, 218, 204),
            );

            ui.add_space(2.0);
            draw_action_buttons(
                ui,
                &mut self.pet,
                &mut self.save_dirty,
                &mut self.message,
                &mut self.message_until,
            );
        });
    }

    fn on_exit(&mut self) {
        self.save();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pet {
    #[serde(default = "random_name")]
    name: String,
    #[serde(default = "random_personality")]
    personality: Personality,
    #[serde(default = "random_body_shape")]
    body_shape: BodyShape,
    #[serde(default = "random_eye_style")]
    eye_style: EyeStyle,
    #[serde(default = "random_palette")]
    palette: Palette,
    #[serde(default = "random_antenna")]
    antenna: bool,
    food: f32,
    happiness: f32,
    energy: f32,
    cleanliness: f32,
    age_seconds: f32,
}

impl Default for Pet {
    fn default() -> Self {
        Self::random()
    }
}

impl Pet {
    fn random() -> Self {
        let personality = random_personality();
        let (food, happiness, energy, cleanliness) = personality.starting_stats();

        Self {
            name: random_name(),
            personality,
            body_shape: random_body_shape(),
            eye_style: random_eye_style(),
            palette: random_palette(),
            antenna: random_antenna(),
            food,
            happiness,
            energy,
            cleanliness,
            age_seconds: 0.0,
        }
    }

    fn update(&mut self, elapsed: f32) {
        self.age_seconds += elapsed;
        let decay = self.personality.decay();
        self.food = clamp_stat(self.food - elapsed * decay.food);
        self.happiness = clamp_stat(self.happiness - elapsed * decay.happiness);
        self.energy = clamp_stat(self.energy - elapsed * decay.energy);
        self.cleanliness = clamp_stat(self.cleanliness - elapsed * decay.cleanliness);

        if self.food < 25.0 {
            self.happiness = clamp_stat(self.happiness - elapsed * 0.10);
        }
        if self.cleanliness < 25.0 {
            self.happiness = clamp_stat(self.happiness - elapsed * 0.08);
        }
    }

    fn feed(&mut self) {
        self.food = clamp_stat(self.food + 22.0);
        self.energy = clamp_stat(self.energy + 4.0);
        self.cleanliness = clamp_stat(self.cleanliness - 5.0);
    }

    fn play(&mut self) {
        self.happiness = clamp_stat(self.happiness + 19.0);
        self.energy = clamp_stat(self.energy - 13.0);
        self.food = clamp_stat(self.food - 8.0);
    }

    fn nap(&mut self) {
        self.energy = clamp_stat(self.energy + 28.0);
        self.food = clamp_stat(self.food - 5.0);
    }

    fn wash(&mut self) {
        self.cleanliness = clamp_stat(self.cleanliness + 30.0);
        self.happiness = clamp_stat(self.happiness + 3.0);
    }

    fn average(&self) -> f32 {
        (self.food + self.happiness + self.energy + self.cleanliness) / 4.0
    }

    fn day(&self) -> u32 {
        (self.age_seconds / 90.0).floor() as u32 + 1
    }

    fn stage(&self) -> &'static str {
        match self.day() {
            1..=2 => "Baby",
            3..=5 => "Kid",
            _ => "Pal",
        }
    }

    fn mood_line(&self) -> &'static str {
        if self.food < 20.0 {
            "It points at its empty snack meter."
        } else if self.energy < 20.0 {
            "It wobbles sleepily."
        } else if self.cleanliness < 20.0 {
            "It could use a scrub."
        } else if self.happiness < 20.0 {
            "It misses playtime."
        } else if self.average() > 78.0 {
            match self.personality {
                Personality::Chipper => "It practically glows with excitement.",
                Personality::Sleepy => "It looks deeply cozy.",
                Personality::Curious => "It studies you with bright eyes.",
                Personality::Fussy => "It accepts your care with dignity.",
            }
        } else {
            match self.personality {
                Personality::Chipper => "It bounces in place.",
                Personality::Sleepy => "It watches the room quietly.",
                Personality::Curious => "It taps at the screen.",
                Personality::Fussy => "It waits for better service.",
            }
        }
    }

    fn face(&self) -> Face {
        if self.average() < 25.0 {
            Face::Sad
        } else if self.energy < 25.0 {
            Face::Sleepy
        } else if self.happiness > 72.0 {
            Face::Happy
        } else {
            Face::Neutral
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Personality {
    Chipper,
    Sleepy,
    Curious,
    Fussy,
}

impl Personality {
    fn label(self) -> &'static str {
        match self {
            Personality::Chipper => "Chipper",
            Personality::Sleepy => "Sleepy",
            Personality::Curious => "Curious",
            Personality::Fussy => "Fussy",
        }
    }

    fn starting_stats(self) -> (f32, f32, f32, f32) {
        let jitter = || rand::random_range(-5.0..=5.0);
        match self {
            Personality::Chipper => (
                78.0 + jitter(),
                88.0 + jitter(),
                82.0 + jitter(),
                76.0 + jitter(),
            ),
            Personality::Sleepy => (
                80.0 + jitter(),
                72.0 + jitter(),
                94.0 + jitter(),
                78.0 + jitter(),
            ),
            Personality::Curious => (
                76.0 + jitter(),
                82.0 + jitter(),
                86.0 + jitter(),
                74.0 + jitter(),
            ),
            Personality::Fussy => (
                84.0 + jitter(),
                70.0 + jitter(),
                80.0 + jitter(),
                90.0 + jitter(),
            ),
        }
    }

    fn decay(self) -> StatDecay {
        match self {
            Personality::Chipper => StatDecay {
                food: 0.20,
                happiness: 0.09,
                energy: 0.13,
                cleanliness: 0.09,
            },
            Personality::Sleepy => StatDecay {
                food: 0.15,
                happiness: 0.11,
                energy: 0.08,
                cleanliness: 0.08,
            },
            Personality::Curious => StatDecay {
                food: 0.18,
                happiness: 0.13,
                energy: 0.11,
                cleanliness: 0.09,
            },
            Personality::Fussy => StatDecay {
                food: 0.17,
                happiness: 0.14,
                energy: 0.10,
                cleanliness: 0.12,
            },
        }
    }
}

struct StatDecay {
    food: f32,
    happiness: f32,
    energy: f32,
    cleanliness: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum BodyShape {
    Round,
    Tall,
    Squat,
    Pointy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum EyeStyle {
    Dots,
    Tall,
    Wide,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Palette {
    Moss,
    Seafoam,
    Gold,
    Plum,
    Blue,
}

impl Palette {
    fn colors(self) -> (Color32, Color32, Color32) {
        match self {
            Palette::Moss => (
                Color32::from_rgb(35, 50, 36),
                Color32::from_rgb(73, 100, 61),
                Color32::from_rgb(101, 134, 76),
            ),
            Palette::Seafoam => (
                Color32::from_rgb(31, 58, 52),
                Color32::from_rgb(70, 128, 107),
                Color32::from_rgb(111, 174, 145),
            ),
            Palette::Gold => (
                Color32::from_rgb(62, 50, 25),
                Color32::from_rgb(146, 113, 47),
                Color32::from_rgb(205, 163, 73),
            ),
            Palette::Plum => (
                Color32::from_rgb(50, 35, 55),
                Color32::from_rgb(103, 68, 118),
                Color32::from_rgb(145, 95, 160),
            ),
            Palette::Blue => (
                Color32::from_rgb(28, 44, 64),
                Color32::from_rgb(64, 105, 145),
                Color32::from_rgb(96, 146, 188),
            ),
        }
    }
}

#[derive(Clone, Copy)]
enum Face {
    Happy,
    Neutral,
    Sad,
    Sleepy,
}

fn clamp_stat(value: f32) -> f32 {
    value.clamp(0.0, 100.0)
}

fn random_name() -> String {
    const NAMES: &[&str] = &[
        "Mochi", "Pip", "Nomi", "Bop", "Kiki", "Toto", "Zuzu", "Miso", "Pixel", "Lulu", "Bibi",
        "Peb",
    ];
    NAMES[rand::random_range(..NAMES.len())].to_owned()
}

fn random_personality() -> Personality {
    match rand::random_range(0..4) {
        0 => Personality::Chipper,
        1 => Personality::Sleepy,
        2 => Personality::Curious,
        _ => Personality::Fussy,
    }
}

fn random_body_shape() -> BodyShape {
    match rand::random_range(0..4) {
        0 => BodyShape::Round,
        1 => BodyShape::Tall,
        2 => BodyShape::Squat,
        _ => BodyShape::Pointy,
    }
}

fn random_eye_style() -> EyeStyle {
    match rand::random_range(0..3) {
        0 => EyeStyle::Dots,
        1 => EyeStyle::Tall,
        _ => EyeStyle::Wide,
    }
}

fn random_palette() -> Palette {
    match rand::random_range(0..5) {
        0 => Palette::Moss,
        1 => Palette::Seafoam,
        2 => Palette::Gold,
        3 => Palette::Plum,
        _ => Palette::Blue,
    }
}

fn random_antenna() -> bool {
    rand::random_bool(0.35)
}

fn draw_action_buttons(
    ui: &mut egui::Ui,
    pet: &mut Pet,
    save_dirty: &mut bool,
    message: &mut String,
    message_until: &mut Instant,
) {
    let gap = 12.0;
    let button_width = ((ui.available_width() - gap * 3.0) / 4.0).clamp(66.0, 150.0);
    let button_height = (button_width * 0.44).clamp(36.0, 54.0);
    let font_size = (button_width * 0.18).clamp(14.0, 20.0);

    ui.spacing_mut().item_spacing.x = gap;
    ui.horizontal(|ui| {
        if action_button(
            ui,
            "FEED",
            Vec2::new(button_width, button_height),
            font_size,
        )
        .clicked()
        {
            pet.feed();
            *save_dirty = true;
            set_message(message, message_until, "Crunch crunch. Much better.");
        }
        if action_button(
            ui,
            "PLAY",
            Vec2::new(button_width, button_height),
            font_size,
        )
        .clicked()
        {
            pet.play();
            *save_dirty = true;
            set_message(message, message_until, "Bleep! Your pet hops around.");
        }
        if action_button(ui, "NAP", Vec2::new(button_width, button_height), font_size).clicked() {
            pet.nap();
            *save_dirty = true;
            set_message(
                message,
                message_until,
                "A quick snooze restores some spark.",
            );
        }
        if action_button(
            ui,
            "WASH",
            Vec2::new(button_width, button_height),
            font_size,
        )
        .clicked()
        {
            pet.wash();
            *save_dirty = true;
            set_message(message, message_until, "Fresh pixels, clear mind.");
        }
    });
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    version: u32,
    pet: Pet,
}

fn save_path() -> PathBuf {
    ProjectDirs::from("com", "Codex", "RetroPet")
        .map(|dirs| dirs.data_local_dir().join("pet.json"))
        .unwrap_or_else(|| PathBuf::from("retro-pet-save.json"))
}

fn load_pet(path: &PathBuf) -> Option<Pet> {
    let contents = fs::read_to_string(path).ok()?;
    let save_data: SaveData = serde_json::from_str(&contents).ok()?;
    Some(save_data.pet)
}

fn save_pet(path: &PathBuf, pet: &Pet) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let save_data = SaveData {
        version: 1,
        pet: pet.clone(),
    };
    let json = serde_json::to_string_pretty(&save_data).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn set_message(message: &mut String, message_until: &mut Instant, text: &str) {
    *message = text.to_owned();
    *message_until = Instant::now() + Duration::from_secs(4);
}

fn action_button(ui: &mut egui::Ui, label: &str, size: Vec2, font_size: f32) -> egui::Response {
    ui.add_sized(
        size,
        egui::Button::new(
            RichText::new(label)
                .font(FontId::monospace(font_size))
                .color(Color32::from_rgb(230, 244, 197)),
        ),
    )
}

fn stat_bar(ui: &mut egui::Ui, label: &str, value: f32, fill: Color32) {
    ui.horizontal(|ui| {
        ui.set_height(22.0);
        ui.label(
            RichText::new(format!("{label:>6}"))
                .font(FontId::monospace(14.0))
                .color(Color32::from_rgb(183, 207, 154)),
        );

        let desired_size = Vec2::new((ui.available_width() - 48.0).max(120.0), 18.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(2), Color32::from_rgb(30, 38, 31));
        painter.rect_stroke(
            rect,
            CornerRadius::same(2),
            Stroke::new(1.0, Color32::from_rgb(103, 130, 91)),
            StrokeKind::Inside,
        );

        let fill_width = rect.width() * (value / 100.0);
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));
        painter.rect_filled(fill_rect.shrink(2.0), CornerRadius::same(1), fill);

        ui.label(
            RichText::new(format!("{value:>3.0}"))
                .font(FontId::monospace(13.0))
                .color(Color32::from_rgb(198, 229, 159)),
        );
    });
}

fn draw_screen(ui: &mut egui::Ui, pet: &Pet, time: f64) {
    let screen_width = ui.available_width().clamp(300.0, 720.0);
    let desired = Vec2::new(screen_width, screen_width * 0.6);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let shell = Rect::from_min_max(
        rect.min + Vec2::new(4.0, 4.0),
        rect.max - Vec2::new(4.0, 4.0),
    );
    painter.rect_filled(shell, CornerRadius::same(8), Color32::from_rgb(50, 62, 45));
    painter.rect_stroke(
        shell,
        CornerRadius::same(8),
        Stroke::new(3.0, Color32::from_rgb(158, 190, 124)),
        StrokeKind::Inside,
    );

    let bezel_x = (shell.width() * 0.06).clamp(16.0, 32.0);
    let bezel_y = (shell.height() * 0.09).clamp(14.0, 28.0);
    let screen = shell.shrink2(Vec2::new(bezel_x, bezel_y));
    painter.rect_filled(
        screen,
        CornerRadius::same(3),
        Color32::from_rgb(169, 196, 132),
    );
    painter.rect_stroke(
        screen,
        CornerRadius::same(3),
        Stroke::new(3.0, Color32::from_rgb(28, 42, 28)),
        StrokeKind::Inside,
    );

    for i in 0..8 {
        let y = screen.top() + screen.height() * (0.08 + i as f32 * 0.12);
        painter.line_segment(
            [
                Pos2::new(screen.left() + screen.width() * 0.035, y),
                Pos2::new(screen.right() - screen.width() * 0.035, y),
            ],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(55, 78, 49, 28)),
        );
    }

    let pixel = (screen.width() / 62.0).clamp(5.0, 11.0).floor();
    let sprite_size = Vec2::new(13.0 * pixel, 12.0 * pixel);
    let bob = (time * 5.0).sin() as f32 * pixel * 0.65;
    let blink = (time % 4.0) > 3.82;
    draw_pet_sprite(
        &painter,
        Pos2::new(
            screen.center().x - sprite_size.x / 2.0,
            screen.center().y - sprite_size.y / 2.0 + bob,
        ),
        pixel,
        pet,
        blink,
    );

    let shadow = Rect::from_center_size(
        Pos2::new(screen.center().x, screen.bottom() - screen.height() * 0.16),
        Vec2::new(sprite_size.x * 1.55, pixel * 2.0),
    );
    painter.rect_filled(
        shadow,
        CornerRadius::same(4),
        Color32::from_rgba_unmultiplied(37, 55, 37, 70),
    );
}

fn draw_pet_sprite(painter: &egui::Painter, origin: Pos2, pixel: f32, pet: &Pet, blink: bool) {
    let face = pet.face();
    let (ink, body, light) = pet.palette.colors();

    let round_rows = [
        "0001111110000",
        "0011222221000",
        "0112222222100",
        "1122222222210",
        "1222222222221",
        "1222222222221",
        "1222222222221",
        "0122222222210",
        "0012222222100",
        "0001122211000",
        "0011000011000",
        "0110000001100",
    ];
    let tall_rows = [
        "0000111100000",
        "0001222210000",
        "0012222221000",
        "0012222221000",
        "0112222221100",
        "0122222222100",
        "0122222222100",
        "0012222221000",
        "0012222221000",
        "0001222210000",
        "0011000110000",
        "0110000011000",
    ];
    let squat_rows = [
        "0000000000000",
        "0000000000000",
        "0001111110000",
        "0012222221000",
        "0122222222100",
        "1222222222210",
        "1222222222210",
        "0122222222100",
        "0012222221000",
        "0001111110000",
        "0011000110000",
        "0110000011000",
    ];
    let pointy_rows = [
        "0000010000000",
        "0000111000000",
        "0001222100000",
        "0012222210000",
        "0112222211000",
        "1122222221100",
        "1222222222100",
        "0122222221000",
        "0012222210000",
        "0001121100000",
        "0011000110000",
        "0110000011000",
    ];

    let rows = match pet.body_shape {
        BodyShape::Round => &round_rows,
        BodyShape::Tall => &tall_rows,
        BodyShape::Squat => &squat_rows,
        BodyShape::Pointy => &pointy_rows,
    };

    for (y, row) in rows.iter().enumerate() {
        for (x, value) in row.chars().enumerate() {
            let color = match value {
                '1' => ink,
                '2' => body,
                _ => continue,
            };
            pixel_rect(painter, origin, x, y, pixel, color);
        }
    }

    if pet.antenna {
        pixel_rect(painter, origin, 6, 0, pixel, ink);
        pixel_rect(painter, origin, 6, 1, pixel, ink);
        pixel_rect(painter, origin, 5, 0, pixel, light);
        pixel_rect(painter, origin, 7, 0, pixel, light);
    }

    match pet.body_shape {
        BodyShape::Round => {}
        BodyShape::Tall => {
            pixel_rect(painter, origin, 3, 3, pixel, ink);
            pixel_rect(painter, origin, 9, 3, pixel, ink);
        }
        BodyShape::Squat => {
            pixel_rect(painter, origin, 0, 6, pixel, ink);
            pixel_rect(painter, origin, 11, 6, pixel, ink);
        }
        BodyShape::Pointy => {
            pixel_rect(painter, origin, 2, 4, pixel, ink);
            pixel_rect(painter, origin, 10, 4, pixel, ink);
        }
    }

    pixel_rect(painter, origin, 5, 4, pixel, light);
    pixel_rect(painter, origin, 6, 4, pixel, light);
    pixel_rect(painter, origin, 4, 5, pixel, light);

    draw_eye(
        painter,
        origin,
        4,
        5,
        pixel,
        blink,
        face,
        pet.eye_style,
        ink,
    );
    draw_eye(
        painter,
        origin,
        8,
        5,
        pixel,
        blink,
        face,
        pet.eye_style,
        ink,
    );

    match face {
        Face::Happy => {
            pixel_rect(painter, origin, 5, 8, pixel, ink);
            pixel_rect(painter, origin, 6, 9, pixel, ink);
            pixel_rect(painter, origin, 7, 9, pixel, ink);
            pixel_rect(painter, origin, 8, 8, pixel, ink);
        }
        Face::Neutral => {
            pixel_rect(painter, origin, 5, 8, pixel, ink);
            pixel_rect(painter, origin, 6, 8, pixel, ink);
            pixel_rect(painter, origin, 7, 8, pixel, ink);
            pixel_rect(painter, origin, 8, 8, pixel, ink);
        }
        Face::Sad => {
            pixel_rect(painter, origin, 5, 9, pixel, ink);
            pixel_rect(painter, origin, 6, 8, pixel, ink);
            pixel_rect(painter, origin, 7, 8, pixel, ink);
            pixel_rect(painter, origin, 8, 9, pixel, ink);
        }
        Face::Sleepy => {
            pixel_rect(painter, origin, 5, 8, pixel, ink);
            pixel_rect(painter, origin, 6, 8, pixel, ink);
            pixel_rect(painter, origin, 8, 8, pixel, ink);
            pixel_rect(painter, origin, 9, 8, pixel, ink);
        }
    }
}

fn draw_eye(
    painter: &egui::Painter,
    origin: Pos2,
    x: usize,
    y: usize,
    pixel: f32,
    blink: bool,
    face: Face,
    eye_style: EyeStyle,
    ink: Color32,
) {
    match (blink, face) {
        (true, _) | (_, Face::Sleepy) => pixel_rect(painter, origin, x, y + 1, pixel, ink),
        _ => match eye_style {
            EyeStyle::Dots => pixel_rect(painter, origin, x, y + 1, pixel, ink),
            EyeStyle::Tall => {
                pixel_rect(painter, origin, x, y, pixel, ink);
                pixel_rect(painter, origin, x, y + 1, pixel, ink);
            }
            EyeStyle::Wide => {
                pixel_rect(painter, origin, x, y + 1, pixel, ink);
                pixel_rect(painter, origin, x + 1, y + 1, pixel, ink);
            }
        },
    }
}

fn pixel_rect(
    painter: &egui::Painter,
    origin: Pos2,
    x: usize,
    y: usize,
    pixel: f32,
    color: Color32,
) {
    let min = Pos2::new(origin.x + x as f32 * pixel, origin.y + y as f32 * pixel);
    painter.rect_filled(
        Rect::from_min_size(min, Vec2::splat(pixel)),
        CornerRadius::ZERO,
        color,
    );
}
