use eframe::egui::{
    self, Align2, Color32, FontId, Key, Rect, RichText, Stroke, StrokeKind, pos2, vec2,
};
use sheetcraft::{
    BORDER_COLOR, BorderPreset, Borders, CellFormat, CellRef, CellValue, NumberFormat, Rgb, Sheet,
    formula, io,
};

const ROWS: usize = 1_000;
const COLUMNS: usize = 26;
const HEADER: f32 = 46.0;
const DEFAULT_COLUMN_WIDTH: u16 = 104;
const DEFAULT_ROW_HEIGHT: u16 = 24;
const MIN_COLUMN_WIDTH: u16 = 48;
const MAX_COLUMN_WIDTH: u16 = 480;
const MIN_ROW_HEIGHT: u16 = 18;
const MAX_ROW_HEIGHT: u16 = 160;

const BACKGROUND: Color32 = Color32::from_rgb(9, 13, 24);
const PANEL: Color32 = Color32::from_rgb(15, 23, 42);
const HEADER_FILL: Color32 = Color32::from_rgb(17, 28, 51);
const GRID_LINE: Color32 = Color32::from_rgb(30, 41, 59);
const TEXT: Color32 = Color32::from_rgb(226, 232, 240);
const MUTED: Color32 = Color32::from_rgb(148, 163, 184);
const ACCENT: Color32 = Color32::from_rgb(52, 211, 153);
const SELECTED: Color32 = Color32::from_rgb(6, 78, 59);
const RANGE_SELECTED: Color32 = Color32::from_rgba_premultiplied(4, 49, 37, 170);
const REFERENCE: Color32 = Color32::from_rgba_premultiplied(8, 31, 23, 38);
const ERROR: Color32 = Color32::from_rgb(248, 113, 113);

const TEXT_COLORS: [(&str, Rgb); 5] = [
    ("Red", Rgb::new(248, 113, 113)),
    ("Amber", Rgb::new(251, 191, 36)),
    ("Green", Rgb::new(52, 211, 153)),
    ("Blue", Rgb::new(96, 165, 250)),
    ("Violet", Rgb::new(192, 132, 252)),
];

const FILL_COLORS: [(&str, Rgb); 4] = [
    ("Green", Rgb::new(6, 78, 59)),
    ("Navy", Rgb::new(30, 58, 138)),
    ("Wine", Rgb::new(127, 29, 29)),
    ("Slate", Rgb::new(51, 65, 85)),
];

const BORDER_PRESETS: [BorderPreset; 4] = [
    BorderPreset::None,
    BorderPreset::All,
    BorderPreset::Outer,
    BorderPreset::Bottom,
];

const NUMBER_FORMATS: [(NumberFormat, &str); 6] = [
    (NumberFormat::General, "General"),
    (NumberFormat::Number { decimals: 0 }, "Number, no decimals"),
    (NumberFormat::Number { decimals: 2 }, "Number, 2 decimals"),
    (NumberFormat::Currency { decimals: 2 }, "Currency"),
    (
        NumberFormat::Currency { decimals: 0 },
        "Currency, no decimals",
    ),
    (NumberFormat::Percent { decimals: 0 }, "Percent"),
];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("SheetCraft"),
        ..Default::default()
    };
    eframe::run_native(
        "SheetCraft",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[derive(Clone, Copy)]
enum ResizeTarget {
    Column(usize),
    Row(usize),
}

struct GridMetrics {
    column_offsets: Vec<f32>,
    row_offsets: Vec<f32>,
}

impl GridMetrics {
    fn new(sheet: &Sheet) -> Self {
        let mut column_offsets = Vec::with_capacity(COLUMNS + 1);
        column_offsets.push(0.0);
        for column in 1..=COLUMNS {
            let width = sheet
                .column_width(column as u32)
                .unwrap_or(DEFAULT_COLUMN_WIDTH);
            let last = *column_offsets.last().unwrap_or(&0.0);
            column_offsets.push(last + f32::from(width));
        }
        let mut row_offsets = Vec::with_capacity(ROWS + 1);
        row_offsets.push(0.0);
        for row in 1..=ROWS {
            let height = sheet.row_height(row as u32).unwrap_or(DEFAULT_ROW_HEIGHT);
            let last = *row_offsets.last().unwrap_or(&0.0);
            row_offsets.push(last + f32::from(height));
        }
        Self {
            column_offsets,
            row_offsets,
        }
    }

    fn column_width(&self, column: usize) -> f32 {
        self.column_offsets[column + 1] - self.column_offsets[column]
    }

    fn row_height(&self, row: usize) -> f32 {
        self.row_offsets[row + 1] - self.row_offsets[row]
    }
}

fn first_visible(offsets: &[f32], position: f32) -> usize {
    offsets
        .partition_point(|offset| *offset <= position)
        .saturating_sub(1)
}

fn last_visible(offsets: &[f32], position: f32) -> usize {
    offsets.partition_point(|offset| *offset < position)
}

fn cell_rect(metrics: &GridMetrics, origin: egui::Pos2, row: usize, column: usize) -> Rect {
    Rect::from_min_max(
        pos2(
            origin.x + metrics.column_offsets[column],
            origin.y + metrics.row_offsets[row],
        ),
        pos2(
            origin.x + metrics.column_offsets[column + 1],
            origin.y + metrics.row_offsets[row + 1],
        ),
    )
}

fn range_rect(
    metrics: &GridMetrics,
    origin: egui::Pos2,
    top: usize,
    left: usize,
    bottom: usize,
    right: usize,
) -> Rect {
    Rect::from_min_max(
        pos2(
            origin.x + metrics.column_offsets[left],
            origin.y + metrics.row_offsets[top],
        ),
        pos2(
            origin.x + metrics.column_offsets[right + 1],
            origin.y + metrics.row_offsets[bottom + 1],
        ),
    )
}

struct App {
    sheet: Sheet,
    selected: CellRef,
    anchor: CellRef,
    formula: String,
    path: String,
    status: String,
    status_ok: bool,
    undo: Vec<Sheet>,
    redo: Vec<Sheet>,
    focus_formula: bool,
    resize_snapshot: Option<Sheet>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BACKGROUND;
        visuals.window_fill = PANEL;
        visuals.extreme_bg_color = PANEL;
        visuals.override_text_color = Some(TEXT);
        visuals.selection.bg_fill = ACCENT;
        cc.egui_ctx.set_visuals(visuals);

        let selected = CellRef::new(1, 1);
        let sheet = demo_sheet();
        let formula = sheet.raw(&selected).unwrap_or_default().to_string();
        Self {
            sheet,
            selected,
            anchor: selected,
            formula,
            path: "budget.xlsx".to_string(),
            status: "Ready. Arrow keys move, Enter applies, Delete clears.".to_string(),
            status_ok: true,
            undo: Vec::new(),
            redo: Vec::new(),
            focus_formula: false,
            resize_snapshot: None,
        }
    }

    fn sync_formula(&mut self) {
        self.formula = self
            .sheet
            .raw(&self.selected)
            .unwrap_or_default()
            .to_string();
    }

    fn commit(&mut self) {
        let before = self.sheet.clone();
        if self.formula.is_empty() {
            self.sheet.clear(&self.selected);
            self.set_status(format!("Cleared {}", self.selected), true);
        } else {
            self.sheet.set(self.selected, self.formula.clone());
            self.set_status(format!("Updated {}", self.selected), true);
        }
        self.record(before);
    }

    fn record(&mut self, before: Sheet) {
        if before == self.sheet {
            return;
        }
        self.undo.push(before);
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn undo(&mut self) {
        let Some(previous) = self.undo.pop() else {
            self.set_status("Nothing to undo".to_string(), false);
            return;
        };
        let current = std::mem::replace(&mut self.sheet, previous);
        self.redo.push(current);
        self.sync_formula();
        self.set_status(format!("Undid change at {}", self.selected), true);
    }

    fn redo(&mut self) {
        let Some(next) = self.redo.pop() else {
            self.set_status("Nothing to redo".to_string(), false);
            return;
        };
        let current = std::mem::replace(&mut self.sheet, next);
        self.undo.push(current);
        self.sync_formula();
        self.set_status(format!("Redid change at {}", self.selected), true);
    }

    fn selection_bounds(&self) -> (u32, u32, u32, u32) {
        (
            self.anchor.row.min(self.selected.row),
            self.anchor.col.min(self.selected.col),
            self.anchor.row.max(self.selected.row),
            self.anchor.col.max(self.selected.col),
        )
    }

    fn selection_label(&self) -> String {
        if self.anchor == self.selected {
            self.selected.to_string()
        } else {
            let (top, left, bottom, right) = self.selection_bounds();
            format!(
                "{}:{}",
                CellRef::new(top, left),
                CellRef::new(bottom, right)
            )
        }
    }

    fn copy(&mut self, ui: &mut egui::Ui) {
        let (top, left, bottom, right) = self.selection_bounds();
        let mut rows = Vec::new();
        for row in top..=bottom {
            let mut fields = Vec::new();
            for column in left..=right {
                let reference = CellRef::new(row, column);
                fields.push(self.sheet.raw(&reference).unwrap_or_default().to_string());
            }
            rows.push(fields.join("\t"));
        }
        ui.ctx().copy_text(rows.join("\n"));
        self.set_status(format!("Copied {}", self.selection_label()), true);
    }

    fn paste(&mut self, text: &str) {
        let before = self.sheet.clone();
        let mut count = 0;
        for (row_offset, line) in text.lines().enumerate() {
            let row = self.selected.row as usize + row_offset;
            if row > ROWS {
                break;
            }
            for (column_offset, field) in line.split('\t').enumerate() {
                let column = self.selected.col as usize + column_offset;
                if column > COLUMNS {
                    break;
                }
                let reference = CellRef::new(row as u32, column as u32);
                if field.is_empty() {
                    self.sheet.clear(&reference);
                } else {
                    self.sheet.set(reference, field);
                }
                count += 1;
            }
        }
        self.record(before);
        self.sync_formula();
        self.set_status(format!("Pasted {count} cells at {}", self.selected), true);
    }

    fn open(&mut self) {
        match io::load(&self.path) {
            Ok(sheet) => {
                self.sheet = sheet;
                self.selected = CellRef::new(1, 1);
                self.anchor = self.selected;
                self.undo.clear();
                self.redo.clear();
                self.sync_formula();
                self.set_status(format!("Opened {}", self.path), true);
            }
            Err(error) => self.set_status(format!("Open failed: {error}"), false),
        }
    }

    fn save(&mut self) {
        match io::save(&self.sheet, &self.path) {
            Ok(()) => self.set_status(format!("Saved {}", self.path), true),
            Err(error) => self.set_status(format!("Save failed: {error}"), false),
        }
    }

    fn set_status(&mut self, message: String, ok: bool) {
        self.status = message;
        self.status_ok = ok;
    }

    fn apply_format(&mut self, message: &str, update: impl Fn(&mut CellFormat)) {
        let before = self.sheet.clone();
        let (top, left, bottom, right) = self.selection_bounds();
        for row in top..=bottom {
            for column in left..=right {
                let reference = CellRef::new(row, column);
                let mut format = self.sheet.format(&reference);
                update(&mut format);
                self.sheet.set_format(reference, format);
            }
        }
        self.record(before);
        self.set_status(format!("{message} {}", self.selection_label()), true);
    }

    fn toggle_bold(&mut self) {
        let bold = !self.sheet.format(&self.selected).bold;
        let message = if bold { "Bold" } else { "Removed bold from" };
        self.apply_format(message, |format| format.bold = bold);
    }

    fn set_text_color(&mut self, color: Option<Rgb>) {
        let message = if color.is_some() {
            "Applied text color to"
        } else {
            "Reset text color on"
        };
        self.apply_format(message, |format| format.text_color = color);
    }

    fn set_fill_color(&mut self, color: Option<Rgb>) {
        let message = if color.is_some() {
            "Applied fill to"
        } else {
            "Removed fill from"
        };
        self.apply_format(message, |format| format.fill_color = color);
    }

    fn set_number_format(&mut self, number_format: NumberFormat) {
        let label = number_format_label(number_format);
        self.apply_format(&format!("Applied {label} to"), |format| {
            format.number_format = number_format;
        });
    }

    fn set_borders(&mut self, preset: BorderPreset) {
        let before = self.sheet.clone();
        let (top, left, bottom, right) = self.selection_bounds();
        for row in top..=bottom {
            for column in left..=right {
                let reference = CellRef::new(row, column);
                let mut format = self.sheet.format(&reference);
                format.borders =
                    preset.borders(row == top, column == left, row == bottom, column == right);
                self.sheet.set_format(reference, format);
            }
        }
        self.record(before);
        self.set_status(
            format!("{} {}", preset.label(), self.selection_label()),
            true,
        );
    }

    fn move_selection(&mut self, row_delta: isize, column_delta: isize, extend: bool) {
        let row = (self.selected.row as isize + row_delta).clamp(1, ROWS as isize);
        let column = (self.selected.col as isize + column_delta).clamp(1, COLUMNS as isize);
        self.selected = CellRef::new(row as u32, column as u32);
        if !extend {
            self.anchor = self.selected;
        }
        self.sync_formula();
    }

    fn handle_keyboard(&mut self, ui: &mut egui::Ui) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let extend = ui.input(|input| input.modifiers.shift);
        let typed = ui.input(|input| {
            if input.modifiers.command || input.modifiers.ctrl || input.modifiers.alt {
                return String::new();
            }
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::Text(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect()
        });
        if !typed.is_empty() {
            self.formula = typed;
            self.focus_formula = true;
            self.set_status(format!("Editing {}", self.selected), true);
        }
        if ui.input(|input| input.key_pressed(Key::F2)) {
            self.focus_formula = true;
            self.set_status(format!("Editing {}", self.selected), true);
        }
        if ui.input(|input| input.key_pressed(Key::ArrowUp)) {
            self.move_selection(-1, 0, extend);
        }
        if ui.input(|input| input.key_pressed(Key::ArrowDown)) {
            self.move_selection(1, 0, extend);
        }
        if ui.input(|input| input.key_pressed(Key::ArrowLeft)) {
            self.move_selection(0, -1, extend);
        }
        if ui.input(|input| input.key_pressed(Key::ArrowRight)) {
            self.move_selection(0, 1, extend);
        }
        if ui.input(|input| input.key_pressed(Key::Delete)) {
            let before = self.sheet.clone();
            let (top, left, bottom, right) = self.selection_bounds();
            for row in top..=bottom {
                for column in left..=right {
                    self.sheet.clear(&CellRef::new(row, column));
                }
            }
            self.formula.clear();
            self.record(before);
            self.set_status(format!("Cleared {}", self.selection_label()), true);
        }
        if ui.input(|input| input.modifiers.command && input.key_pressed(Key::C)) {
            self.copy(ui);
        }
        if let Some(text) = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Paste(text) => Some(text.clone()),
                _ => None,
            })
        }) {
            self.paste(&text);
        }
        if ui.input(|input| input.modifiers.command && input.key_pressed(Key::Z)) {
            if ui.input(|input| input.modifiers.shift) {
                self.redo();
            } else {
                self.undo();
            }
        }
        if ui.input(|input| input.modifiers.command && input.key_pressed(Key::Y)) {
            self.redo();
        }
        if ui.input(|input| input.modifiers.command && input.key_pressed(Key::S)) {
            self.save();
        }
        if ui.input(|input| input.modifiers.command && input.key_pressed(Key::B)) {
            self.toggle_bold();
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("toolbar")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("SheetCraft")
                            .size(22.0)
                            .strong()
                            .color(ACCENT),
                    );
                    ui.label(RichText::new("offline spreadsheet").small().color(MUTED));
                    ui.separator();
                    ui.label(RichText::new("File").color(MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.path)
                            .desired_width(320.0)
                            .hint_text("workbook.csv or workbook.xlsx"),
                    );
                    if ui.button("Open").clicked() {
                        self.open();
                    }
                    if ui.button("Save").clicked() {
                        self.save();
                    }
                    if ui
                        .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        self.undo();
                    }
                    if ui
                        .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                        .clicked()
                    {
                        self.redo();
                    }
                    ui.separator();
                    ui.label(
                        RichText::new(format!("Cell {}", self.selection_label()))
                            .monospace()
                            .color(ACCENT),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("fx").strong().color(ACCENT));
                    let width = (ui.available_width() - 84.0).max(120.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.formula)
                            .desired_width(width)
                            .font(egui::TextStyle::Monospace)
                            .hint_text("Enter a value or formula, then press Enter"),
                    );
                    if self.focus_formula {
                        response.request_focus();
                        let cursor = egui::text::CCursor::new(self.formula.chars().count());
                        if let Some(mut state) =
                            egui::text_edit::TextEditState::load(ui.ctx(), response.id)
                        {
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::one(cursor)));
                            state.store(ui.ctx(), response.id);
                        }
                        self.focus_formula = false;
                    }
                    if response.has_focus() && ui.input(|input| input.key_pressed(Key::Escape)) {
                        self.sync_formula();
                        response.surrender_focus();
                    }
                    if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                        self.commit();
                        self.move_selection(1, 0, false);
                    }
                    if ui.button("Apply").clicked() {
                        self.commit();
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let selected_format = self.sheet.format(&self.selected);
                    if ui
                        .add(
                            egui::Button::new(RichText::new("B").strong())
                                .selected(selected_format.bold),
                        )
                        .on_hover_text("Bold (Ctrl/Cmd+B)")
                        .clicked()
                    {
                        self.toggle_bold();
                    }
                    ui.separator();
                    ui.label(RichText::new("Text").color(MUTED));
                    if ui
                        .small_button("Auto")
                        .on_hover_text("Default text color")
                        .clicked()
                    {
                        self.set_text_color(None);
                    }
                    for (name, color) in TEXT_COLORS {
                        if color_swatch(ui, color, selected_format.text_color == Some(color), name)
                        {
                            self.set_text_color(Some(color));
                        }
                    }
                    ui.separator();
                    ui.label(RichText::new("Fill").color(MUTED));
                    if ui
                        .small_button("None")
                        .on_hover_text("Remove fill color")
                        .clicked()
                    {
                        self.set_fill_color(None);
                    }
                    for (name, color) in FILL_COLORS {
                        if color_swatch(ui, color, selected_format.fill_color == Some(color), name)
                        {
                            self.set_fill_color(Some(color));
                        }
                    }
                    ui.separator();
                    ui.label(RichText::new("Borders").color(MUTED));
                    egui::ComboBox::from_id_salt("borders")
                        .selected_text("Apply")
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for preset in BORDER_PRESETS {
                                if ui.selectable_label(false, preset.label()).clicked() {
                                    self.set_borders(preset);
                                }
                            }
                        });
                    ui.separator();
                    ui.label(RichText::new("Number").color(MUTED));
                    egui::ComboBox::from_id_salt("number-format")
                        .selected_text(number_format_label(selected_format.number_format))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for (number_format, label) in NUMBER_FORMATS {
                                if ui
                                    .selectable_label(
                                        selected_format.number_format == number_format,
                                        label,
                                    )
                                    .clicked()
                                {
                                    self.set_number_format(number_format);
                                }
                            }
                        });
                });
            });
    }

    fn grid(&mut self, ui: &mut egui::Ui) {
        let metrics = GridMetrics::new(&self.sheet);
        let sheet = &self.sheet;
        let selected = self.selected;
        let (top, left, bottom, right) = self.selection_bounds();
        let references = formula::references(&self.formula);
        let mut clicked = None;
        let mut edit_requested = false;
        let mut resize_started = false;
        let mut resize_stopped = false;
        let mut resize_update = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BACKGROUND))
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt("sheet-grid")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let size = vec2(
                            HEADER + metrics.column_offsets[COLUMNS],
                            HEADER + metrics.row_offsets[ROWS],
                        );
                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 0.0, BACKGROUND);

                        let clip = ui.clip_rect().intersect(rect);
                        let body_left = rect.left() + HEADER;
                        let body_top = rect.top() + HEADER;
                        let body_origin = pos2(body_left, body_top);

                        let first_row = first_visible(&metrics.row_offsets, clip.top() - body_top)
                            .min(ROWS - 1);
                        let last_row = last_visible(&metrics.row_offsets, clip.bottom() - body_top)
                            .clamp(first_row + 1, ROWS);
                        let first_column =
                            first_visible(&metrics.column_offsets, clip.left() - body_left)
                                .min(COLUMNS - 1);
                        let last_column =
                            last_visible(&metrics.column_offsets, clip.right() - body_left)
                                .clamp(first_column + 1, COLUMNS);

                        painter.rect_filled(
                            Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + HEADER)),
                            0.0,
                            HEADER_FILL,
                        );
                        painter.rect_filled(
                            Rect::from_min_max(rect.min, pos2(rect.left() + HEADER, rect.bottom())),
                            0.0,
                            HEADER_FILL,
                        );

                        for column in first_column..last_column {
                            let x = body_left + metrics.column_offsets[column];
                            let width = metrics.column_width(column);
                            painter.text(
                                pos2(x + width / 2.0, rect.top() + HEADER / 2.0),
                                Align2::CENTER_CENTER,
                                CellRef::column_name(column as u32 + 1),
                                FontId::proportional(12.0),
                                MUTED,
                            );
                        }
                        for row in first_row..last_row {
                            let y = body_top + metrics.row_offsets[row];
                            let height = metrics.row_height(row);
                            painter.text(
                                pos2(rect.left() + HEADER / 2.0, y + height / 2.0),
                                Align2::CENTER_CENTER,
                                (row + 1).to_string(),
                                FontId::proportional(12.0),
                                MUTED,
                            );
                        }

                        for row in first_row..last_row {
                            for column in first_column..last_column {
                                let reference = CellRef::new(row as u32 + 1, column as u32 + 1);
                                let format = sheet.format(&reference);
                                let cell = cell_rect(&metrics, body_origin, row, column);
                                if let Some(fill) = format.fill_color {
                                    painter.rect_filled(cell, 0.0, color32(fill));
                                }
                                let value = sheet.value(&reference);
                                let text = display_value(&value, format.number_format);
                                if text.is_empty() {
                                    continue;
                                }
                                let color = format
                                    .text_color
                                    .map(color32)
                                    .unwrap_or_else(|| if value.is_error() { ERROR } else { TEXT });
                                let position = cell.min + vec2(6.0, 4.0);
                                draw_text(&painter, position, &text, color, format.bold);
                            }
                        }

                        for column in first_column..=last_column {
                            let x = body_left + metrics.column_offsets[column];
                            painter.line_segment(
                                [pos2(x, rect.top() + HEADER), pos2(x, rect.bottom())],
                                Stroke::new(1.0, GRID_LINE),
                            );
                        }
                        for row in first_row..=last_row {
                            let y = body_top + metrics.row_offsets[row];
                            painter.line_segment(
                                [pos2(rect.left() + HEADER, y), pos2(rect.right(), y)],
                                Stroke::new(1.0, GRID_LINE),
                            );
                        }

                        for row in first_row..last_row {
                            for column in first_column..last_column {
                                let reference = CellRef::new(row as u32 + 1, column as u32 + 1);
                                let borders = sheet.format(&reference).borders;
                                if borders.is_default() {
                                    continue;
                                }
                                let cell = cell_rect(&metrics, body_origin, row, column);
                                let min = cell.min;
                                let max = cell.max;
                                let stroke = Stroke::new(1.5, color32(BORDER_COLOR));
                                if borders.top {
                                    painter.line_segment([min, pos2(max.x, min.y)], stroke);
                                }
                                if borders.right {
                                    painter.line_segment([pos2(max.x, min.y), max], stroke);
                                }
                                if borders.bottom {
                                    painter.line_segment([pos2(min.x, max.y), max], stroke);
                                }
                                if borders.left {
                                    painter.line_segment([min, pos2(min.x, max.y)], stroke);
                                }
                            }
                        }

                        for reference in &references {
                            let (start, end) = match *reference {
                                formula::Reference::Cell(reference) => (reference, reference),
                                formula::Reference::Range(start, end) => (start, end),
                            };
                            let top = start.row.min(end.row) as usize;
                            let bottom = start.row.max(end.row) as usize;
                            let left = start.col.min(end.col) as usize;
                            let right = start.col.max(end.col) as usize;
                            if top > ROWS || left > COLUMNS {
                                continue;
                            }
                            let cell = range_rect(
                                &metrics,
                                body_origin,
                                (top - 1).min(ROWS - 1),
                                (left - 1).min(COLUMNS - 1),
                                (bottom - 1).min(ROWS - 1),
                                (right - 1).min(COLUMNS - 1),
                            );
                            painter.rect_filled(cell, 0.0, REFERENCE);
                        }

                        let selection = range_rect(
                            &metrics,
                            body_origin,
                            (top - 1) as usize,
                            (left - 1) as usize,
                            (bottom - 1) as usize,
                            (right - 1) as usize,
                        );
                        let fill = if top == bottom && left == right {
                            SELECTED
                        } else {
                            RANGE_SELECTED
                        };
                        painter.rect_filled(selection, 0.0, fill);
                        painter.rect_stroke(
                            selection,
                            0.0,
                            Stroke::new(1.0, ACCENT),
                            StrokeKind::Inside,
                        );

                        let active = cell_rect(
                            &metrics,
                            body_origin,
                            selected.row as usize - 1,
                            selected.col as usize - 1,
                        );
                        let active_format = sheet.format(&selected);
                        let active_value = sheet.value(&selected);
                        let active_text = display_value(&active_value, active_format.number_format);
                        if !active_text.is_empty() {
                            draw_text(
                                &painter,
                                active.min + vec2(6.0, 4.0),
                                &active_text,
                                Color32::WHITE,
                                active_format.bold,
                            );
                        }

                        for column in first_column..last_column {
                            let x = body_left + metrics.column_offsets[column + 1];
                            let handle = Rect::from_center_size(
                                pos2(x, rect.top() + HEADER / 2.0),
                                vec2(7.0, HEADER),
                            );
                            let handle_response = ui
                                .interact(
                                    handle,
                                    ui.scope_id().with(("column-resize", column)),
                                    egui::Sense::drag(),
                                )
                                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                            if handle_response.drag_started() {
                                resize_started = true;
                            }
                            if handle_response.dragged() {
                                resize_update = Some((
                                    ResizeTarget::Column(column),
                                    handle_response.drag_delta().x,
                                ));
                            }
                            if handle_response.drag_stopped() {
                                resize_stopped = true;
                            }
                        }
                        for row in first_row..last_row {
                            let y = body_top + metrics.row_offsets[row + 1];
                            let handle = Rect::from_center_size(
                                pos2(rect.left() + HEADER / 2.0, y),
                                vec2(HEADER, 7.0),
                            );
                            let handle_response = ui
                                .interact(
                                    handle,
                                    ui.scope_id().with(("row-resize", row)),
                                    egui::Sense::drag(),
                                )
                                .on_hover_cursor(egui::CursorIcon::ResizeVertical);
                            if handle_response.drag_started() {
                                resize_started = true;
                            }
                            if handle_response.dragged() {
                                resize_update =
                                    Some((ResizeTarget::Row(row), handle_response.drag_delta().y));
                            }
                            if handle_response.drag_stopped() {
                                resize_stopped = true;
                            }
                        }

                        if response.clicked()
                            && let Some(position) = response.interact_pointer_pos()
                        {
                            let relative = position - rect.min;
                            if relative.x > HEADER && relative.y > HEADER {
                                let row = first_visible(&metrics.row_offsets, relative.y - HEADER)
                                    .min(ROWS - 1);
                                let column =
                                    first_visible(&metrics.column_offsets, relative.x - HEADER)
                                        .min(COLUMNS - 1);
                                let extend = ui.input(|input| input.modifiers.shift);
                                clicked =
                                    Some((CellRef::new(row as u32 + 1, column as u32 + 1), extend));
                                edit_requested = response.double_clicked();
                            }
                        }
                    });
            });

        if resize_started {
            self.resize_snapshot = Some(self.sheet.clone());
        }
        if let Some((target, delta)) = resize_update {
            let message = match target {
                ResizeTarget::Column(column) => {
                    let current = f32::from(
                        self.sheet
                            .column_width(column as u32 + 1)
                            .unwrap_or(DEFAULT_COLUMN_WIDTH),
                    );
                    let width = (current + delta)
                        .round()
                        .clamp(f32::from(MIN_COLUMN_WIDTH), f32::from(MAX_COLUMN_WIDTH))
                        as u16;
                    self.sheet.set_column_width(
                        column as u32 + 1,
                        (width != DEFAULT_COLUMN_WIDTH).then_some(width),
                    );
                    format!(
                        "Column {} width {} px",
                        CellRef::column_name(column as u32 + 1),
                        width
                    )
                }
                ResizeTarget::Row(row) => {
                    let current = f32::from(
                        self.sheet
                            .row_height(row as u32 + 1)
                            .unwrap_or(DEFAULT_ROW_HEIGHT),
                    );
                    let height = (current + delta)
                        .round()
                        .clamp(f32::from(MIN_ROW_HEIGHT), f32::from(MAX_ROW_HEIGHT))
                        as u16;
                    self.sheet.set_row_height(
                        row as u32 + 1,
                        (height != DEFAULT_ROW_HEIGHT).then_some(height),
                    );
                    format!("Row {} height {} px", row + 1, height)
                }
            };
            self.set_status(message, true);
        }
        if resize_stopped && let Some(before) = self.resize_snapshot.take() {
            self.record(before);
        }

        if let Some((reference, extend)) = clicked {
            self.selected = reference;
            if !extend {
                self.anchor = reference;
            }
            self.sync_formula();
            if edit_requested {
                self.focus_formula = true;
                self.set_status(format!("Editing {}", self.selected), true);
            }
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let color = if self.status_ok { ACCENT } else { ERROR };
                    ui.label(RichText::new(&self.status).size(12.0).color(color));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(
                                "F2 edits  |  Drag headers resizes  |  Shift+Arrows selects  |  Ctrl/Cmd+B bold  |  Ctrl/Cmd+C/V/Z  |  Ctrl/Cmd+S saves",
                            )
                            .size(12.0)
                            .color(MUTED),
                        );
                    });
                });
            });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_keyboard(ui);
        self.toolbar(ui);
        self.status_bar(ui);
        self.grid(ui);
    }
}

fn demo_sheet() -> Sheet {
    let mut sheet = Sheet::new();
    let cells = [
        ("A1", "Item"),
        ("B1", "Cost"),
        ("C1", "Qty"),
        ("D1", "Total"),
        ("A2", "Keyboard"),
        ("B2", "79"),
        ("C2", "2"),
        ("D2", "=B2*C2"),
        ("A3", "Monitor"),
        ("B3", "249"),
        ("C3", "1"),
        ("D3", "=B3*C3"),
        ("A4", "Dock"),
        ("B4", "129"),
        ("C4", "1"),
        ("D4", "=B4*C4"),
        ("A5", "Total"),
        ("D5", "=SUM(D2:D4)"),
    ];
    for (reference, raw) in cells {
        sheet.set_a1(reference, raw).unwrap();
    }
    sheet.set_column_width(1, Some(150));
    sheet.set_column_width(2, Some(100));
    sheet.set_column_width(3, Some(70));
    sheet.set_column_width(4, Some(120));
    sheet.set_row_height(1, Some(30));
    let header = CellFormat {
        bold: true,
        text_color: Some(Rgb::new(226, 232, 240)),
        fill_color: Some(Rgb::new(30, 58, 138)),
        borders: Borders::ALL,
        ..CellFormat::default()
    };
    for column in 1..=4 {
        sheet.set_format(CellRef::new(1, column), header);
    }
    let money = CellFormat {
        number_format: NumberFormat::Currency { decimals: 2 },
        ..CellFormat::default()
    };
    for row in 2..=5 {
        sheet.set_format(CellRef::new(row, 2), money);
        sheet.set_format(CellRef::new(row, 4), money);
    }
    sheet.set_format(
        CellRef::new(5, 1),
        CellFormat {
            bold: true,
            ..CellFormat::default()
        },
    );
    sheet.set_format(
        CellRef::new(5, 4),
        CellFormat {
            bold: true,
            number_format: NumberFormat::Currency { decimals: 2 },
            ..CellFormat::default()
        },
    );
    sheet
}

fn number_format_label(number_format: NumberFormat) -> &'static str {
    NUMBER_FORMATS
        .iter()
        .find(|(format, _)| *format == number_format)
        .map(|(_, label)| *label)
        .unwrap_or("Custom")
}

fn display_value(value: &CellValue, number_format: NumberFormat) -> String {
    match value {
        CellValue::Number(number) if !matches!(number_format, NumberFormat::General) => {
            number_format.apply(*number)
        }
        _ => value.to_string(),
    }
}

fn color32(color: Rgb) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

fn color_swatch(ui: &mut egui::Ui, color: Rgb, selected: bool, tooltip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::click());
    ui.painter().rect_filled(rect, 3.0, color32(color));
    if selected {
        ui.painter()
            .rect_stroke(rect, 3.0, Stroke::new(2.0, ACCENT), StrokeKind::Inside);
    }
    response.on_hover_text(tooltip).clicked()
}

fn draw_text(
    painter: &egui::Painter,
    position: egui::Pos2,
    text: &str,
    color: Color32,
    bold: bool,
) {
    painter.text(
        position,
        Align2::LEFT_TOP,
        text,
        FontId::monospace(13.0),
        color,
    );
    if bold {
        painter.text(
            position + vec2(0.6, 0.0),
            Align2::LEFT_TOP,
            text,
            FontId::monospace(13.0),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_use_custom_and_default_sizes() {
        let mut sheet = Sheet::new();
        sheet.set_column_width(2, Some(200));
        sheet.set_row_height(2, Some(40));
        let metrics = GridMetrics::new(&sheet);
        assert_eq!(metrics.column_width(0), f32::from(DEFAULT_COLUMN_WIDTH));
        assert_eq!(metrics.column_width(1), 200.0);
        assert_eq!(metrics.row_height(0), f32::from(DEFAULT_ROW_HEIGHT));
        assert_eq!(metrics.row_height(1), 40.0);
    }

    #[test]
    fn finds_visible_cells_and_rects() {
        let mut sheet = Sheet::new();
        sheet.set_column_width(2, Some(200));
        let metrics = GridMetrics::new(&sheet);
        assert_eq!(first_visible(&metrics.column_offsets, 0.0), 0);
        assert_eq!(first_visible(&metrics.column_offsets, 105.0), 1);
        assert_eq!(first_visible(&metrics.column_offsets, 205.0), 1);
        assert_eq!(last_visible(&metrics.column_offsets, 100.0), 1);
        assert_eq!(last_visible(&metrics.column_offsets, 205.0), 2);

        let origin = pos2(0.0, 0.0);
        let second = cell_rect(&metrics, origin, 0, 1);
        assert_eq!(second.min, pos2(104.0, 0.0));
        assert_eq!(second.max, pos2(304.0, f32::from(DEFAULT_ROW_HEIGHT)));
        let range = range_rect(&metrics, origin, 0, 0, 0, 1);
        assert_eq!(range.max.x, 304.0);
    }
}
