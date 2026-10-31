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
- [ ] Copy and paste
- [ ] Reference highlighting
- [ ] Undo and redo
- [ ] Instant startup benchmark under 100 ms

## v0.4 Everyday use

- [ ] Number formats, bold, colors, borders
- [ ] Column widths and row heights
- [ ] Multiple sheets
- [ ] Sorting and filtering
- [ ] Charts
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
