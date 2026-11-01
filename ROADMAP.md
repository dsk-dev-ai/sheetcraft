# SheetCraft Roadmap

Goal: a fast, offline, open-source spreadsheet editor in pure Rust that is good
enough to replace a browser spreadsheet for everyday work.

## v0.1 Core engine

- [x] Typed cell values
- [x] Formula tokenizer and parser
- [x] Arithmetic, references, ranges, unary operators
- [x] `SUM`, `AVERAGE`, `MIN`, `MAX`
- [x] Circular-reference detection
- [x] CLI for evaluating formulas
- [ ] More functions: `IF`, `COUNT`, `COUNTA`, `ROUND`, `ABS`
- [ ] Comparison operators and strings in formulas

## v0.2 Files

- [x] CSV import and export
- [x] XLSX import with `calamine`
- [x] Import supported formulas from XLSX
- [x] XLSX export with `rust_xlsxwriter`
- [x] Round-trip tests against generated workbooks
- [ ] Large-file benchmark: 100k rows, 20 columns

## v0.3 Native GUI

- [x] `eframe`/`egui` shell
- [x] Virtualized grid renderer
- [x] Cell editing and keyboard navigation
- [x] Formula bar
- [x] Open and save CSV or XLSX from the desktop app
- [x] Range selection with Shift
- [x] Copy and paste
- [x] Reference highlighting
- [x] Undo and redo
- [x] Release-build startup benchmark under 300 ms (about 250 ms locally)

## v0.3 Release

- [x] Cross-platform release archives for the CLI and desktop app
- [x] Tag and publish v0.3.0

## v0.4 Everyday use

- [x] Number formats with thousands separators, currency, and percent
- [x] Bold, text colors, and cell fills
- [x] XLSX export preserves cell formatting
- [x] Cell borders
- [x] Column widths and row heights
- [x] Multiple sheets
- [x] Sorting and filtering
- [x] Charts
- [ ] Freeze panes

## v0.5 Distribution

- [ ] Single-binary builds for Linux, macOS, Windows
- [ ] Package for Homebrew, AUR, and winget
- [ ] Website with a 20-second demo
- [ ] Reproducible benchmark suite

## v1.0 Launch

- [ ] No known data-loss bugs on XLSX round trips
- [ ] Documentation and file-format notes
- [ ] Launch posts prepared for Hacker News, r/rust, r/selfhosted
- [ ] Public changelog and release cadence

## Working rules

1. Keep the default build offline: no network dependencies, no telemetry.
2. Benchmark before optimizing and publish the numbers.
3. Every feature ships with tests.
4. Never break opening an existing file to add a new one.
