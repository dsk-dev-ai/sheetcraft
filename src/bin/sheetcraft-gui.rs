use eframe::egui::{
    self, Align2, Color32, FontId, Key, Rect, RichText, Stroke, StrokeKind, pos2, vec2,
};
use sheetcraft::{CellRef, CellValue, Sheet, formula, io};

const ROWS: usize = 1_000;
const COLUMNS: usize = 26;
const HEADER: f32 = 46.0;
const CELL_WIDTH: f32 = 104.0;
const CELL_HEIGHT: f32 = 24.0;

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
                    if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                        self.commit();
                    }
                    if ui.button("Apply").clicked() {
                        self.commit();
                    }
                });
            });
    }

    fn grid(&mut self, ui: &mut egui::Ui) {
        let sheet = &self.sheet;
        let selected = self.selected;
        let (top, left, bottom, right) = self.selection_bounds();
        let references = formula::references(&self.formula);
        let mut clicked = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BACKGROUND))
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt("sheet-grid")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let size = vec2(
                            HEADER + COLUMNS as f32 * CELL_WIDTH,
                            HEADER + ROWS as f32 * CELL_HEIGHT,
                        );
                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 0.0, BACKGROUND);

                        let clip = ui.clip_rect().intersect(rect);
                        let body_left = rect.left() + HEADER;
                        let body_top = rect.top() + HEADER;

                        let first_row = (((clip.top() - body_top) / CELL_HEIGHT).floor() as isize)
                            .clamp(0, ROWS as isize - 1)
                            as usize;
                        let last_row = (((clip.bottom() - body_top) / CELL_HEIGHT).ceil() as usize)
                            .min(ROWS)
                            .max(first_row + 1);
                        let first_column = (((clip.left() - body_left) / CELL_WIDTH).floor()
                            as isize)
                            .clamp(0, COLUMNS as isize - 1)
                            as usize;
                        let last_column = (((clip.right() - body_left) / CELL_WIDTH).ceil()
                            as usize)
                            .min(COLUMNS)
                            .max(first_column + 1);

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
                            let x = body_left + column as f32 * CELL_WIDTH;
                            painter.text(
                                pos2(x + CELL_WIDTH / 2.0, rect.top() + HEADER / 2.0),
                                Align2::CENTER_CENTER,
                                CellRef::column_name(column as u32 + 1),
                                FontId::proportional(12.0),
                                MUTED,
                            );
                        }
                        for row in first_row..last_row {
                            let y = body_top + row as f32 * CELL_HEIGHT;
                            painter.text(
                                pos2(rect.left() + HEADER / 2.0, y + CELL_HEIGHT / 2.0),
                                Align2::CENTER_CENTER,
                                (row + 1).to_string(),
                                FontId::proportional(12.0),
                                MUTED,
                            );
                        }

                        for row in first_row..last_row {
                            for column in first_column..last_column {
                                let reference = CellRef::new(row as u32 + 1, column as u32 + 1);
                                let value = sheet.value(&reference);
                                let (text, color) = match value {
                                    CellValue::Empty => (String::new(), TEXT),
                                    CellValue::Error(_) => (value.to_string(), ERROR),
                                    other => (other.to_string(), TEXT),
                                };
                                if text.is_empty() {
                                    continue;
                                }
                                let position = pos2(
                                    body_left + column as f32 * CELL_WIDTH + 6.0,
                                    body_top + row as f32 * CELL_HEIGHT + 4.0,
                                );
                                painter.text(
                                    position,
                                    Align2::LEFT_TOP,
                                    text,
                                    FontId::monospace(13.0),
                                    color,
                                );
                            }
                        }

                        for column in first_column..=last_column {
                            let x = body_left + column as f32 * CELL_WIDTH;
                            painter.line_segment(
                                [pos2(x, rect.top() + HEADER), pos2(x, rect.bottom())],
                                Stroke::new(1.0, GRID_LINE),
                            );
                        }
                        for row in first_row..=last_row {
                            let y = body_top + row as f32 * CELL_HEIGHT;
                            painter.line_segment(
                                [pos2(rect.left() + HEADER, y), pos2(rect.right(), y)],
                                Stroke::new(1.0, GRID_LINE),
                            );
                        }

                        for reference in &references {
                            let (start, end) = match *reference {
                                formula::Reference::Cell(reference) => (reference, reference),
                                formula::Reference::Range(start, end) => (start, end),
                            };
                            let top = start.row.min(end.row);
                            let bottom = start.row.max(end.row);
                            let left = start.col.min(end.col);
                            let right = start.col.max(end.col);
                            let cell = Rect::from_min_size(
                                pos2(
                                    body_left + (left - 1) as f32 * CELL_WIDTH,
                                    body_top + (top - 1) as f32 * CELL_HEIGHT,
                                ),
                                vec2(
                                    (right - left + 1) as f32 * CELL_WIDTH,
                                    (bottom - top + 1) as f32 * CELL_HEIGHT,
                                ),
                            );
                            painter.rect_filled(cell, 0.0, REFERENCE);
                        }

                        let selection = Rect::from_min_size(
                            pos2(
                                body_left + (left - 1) as f32 * CELL_WIDTH,
                                body_top + (top - 1) as f32 * CELL_HEIGHT,
                            ),
                            vec2(
                                (right - left + 1) as f32 * CELL_WIDTH,
                                (bottom - top + 1) as f32 * CELL_HEIGHT,
                            ),
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

                        let active = Rect::from_min_size(
                            pos2(
                                body_left + (selected.col as usize - 1) as f32 * CELL_WIDTH,
                                body_top + (selected.row as usize - 1) as f32 * CELL_HEIGHT,
                            ),
                            vec2(CELL_WIDTH, CELL_HEIGHT),
                        );
                        painter.text(
                            active.min + vec2(6.0, 4.0),
                            Align2::LEFT_TOP,
                            sheet.value(&selected).to_string(),
                            FontId::monospace(13.0),
                            Color32::WHITE,
                        );

                        if response.clicked()
                            && let Some(position) = response.interact_pointer_pos()
                        {
                            let relative = position - rect.min;
                            if relative.x > HEADER && relative.y > HEADER {
                                let row = ((relative.y - HEADER) / CELL_HEIGHT).floor() as usize;
                                let column = ((relative.x - HEADER) / CELL_WIDTH).floor() as usize;
                                if row < ROWS && column < COLUMNS {
                                    let extend = ui.input(|input| input.modifiers.shift);
                                    clicked = Some((
                                        CellRef::new(row as u32 + 1, column as u32 + 1),
                                        extend,
                                    ));
                                }
                            }
                        }
                    });
            });

        if let Some((reference, extend)) = clicked {
            self.selected = reference;
            if !extend {
                self.anchor = reference;
            }
            self.sync_formula();
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
                                "Arrows move  |  Enter applies  |  Delete clears  |  Ctrl/Cmd+C/V copy-paste  |  Ctrl/Cmd+Z undo  |  Ctrl/Cmd+S saves",
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
    sheet
}
