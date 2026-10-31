# SheetCraft

An open-source spreadsheet editor written from scratch in pure Rust.

**Fast. Offline. Private. One binary. No account, no subscription, no telemetry.**

SheetCraft aims to be the spreadsheet people reach for when they want Excel-class
editing without a cloud account or a 500 MB office suite: native performance,
instant startup, full local ownership of every file.

## Why this exists

- Spreadsheet work is still dominated by cloud-first tools that require accounts
  and upload private financial data.
- Existing desktop suites are large, slow to start, and carry decades of legacy.
- The "clean-room, pure Rust" approach already proved it can win attention for
  photo, PDF, video, and word editors. A spreadsheet is the largest remaining gap.

## Status

Early core engine. The repository currently ships:

- A cell model with typed values (number, text, boolean, error)
- A formula parser and evaluator with arithmetic, cell refs, ranges, and
  `SUM` / `AVERAGE` / `MIN` / `MAX`
- Cycle detection for circular references
- A small CLI for evaluating formulas and running a demo sheet
- Unit tests for parsing and evaluation

The GUI, file import/export, and formatting are on the roadmap.

## Quick start

```bash
cargo run -- demo
cargo run -- eval '=SUM(A1:A3) * 2'
cargo test
```

## Roadmap

See [ROADMAP.md](ROADMAP.md).

- v0.1 Core engine and CLI (current)
- v0.2 XLSX / CSV import and export
- v0.3 Native GUI with a virtualized grid
- v0.4 Formatting, charts, and multi-sheet workbooks
- v1.0 Stable release and launch

## Design principles

1. **Local-first.** Files stay on disk. No network calls in the default build.
2. **Instant.** Cold start under 100 ms, even on large workbooks.
3. **Correct.** Formula results are tested against reference spreadsheets.
4. **Small.** A single self-contained binary, no runtime install.
5. **Open.** MIT licensed, no open-core bait-and-switch.

## Viral launch plan

Positioning: *"Excel, rebuilt from scratch in pure Rust. Offline, instant, and
free forever."*

1. Ship a genuinely useful v0.3 before the first public post.
2. Record a 20-second GIF: open a 50 MB workbook instantly, edit, save, no
   account prompt.
3. Publish benchmark numbers against LibreOffice and a browser spreadsheet on
   cold start, memory, and large-file load.
4. Launch on Hacker News, r/rust, r/selfhosted, and r/opensource with the demo
   and source in the first line.
5. Keep a public changelog and post a short demo for every release.

Stars follow utility. The first goal is not virality, it is a tool people keep
open on their desktop.

## License

MIT
