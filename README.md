<p align="center">
  <img src="data/icons/hicolor/128x128/apps/io.github.quicknotes.png" width="128" alt="QuickNotes icon">
</p>

# QuickNotes

A small tabbed notepad for Linux, written in Rust with GTK4 / libadwaita.
Notes are saved automatically — no "save as file" required. Everything is back
after a restart.

## Features

- Tabs for notes and opened files; restored on startup (content, cursor, syntax, preview)
- Autosave of notes and of unsaved changes to opened files to `~/.local/share/quicknotes/`
- Syntax highlighting (GtkSourceView, ~150 languages), auto-detected for pasted snippets
- Live preview side-by-side for **Markdown**, **reStructuredText**, **PlantUML** and **HTML**
- Export of Markdown and reStructuredText documents to **HTML**, **PDF**, **images** (SVG, PNG, JPG) and **EPUB**
- **Presentations** in Markdown: slides separated by `---`, slide transitions, step-by-step
  reveal (`::: steps fade-up`), positioning (`::: place bottom right`, columns), background
  images, presenter notes (`???`); presenter view with notes, next slide and timer on a second
  screen; export as stand-alone HTML or PDF
- Image viewer (PNG, JPEG, GIF, WebP, SVG, BMP, ICO, TIFF, …) with zoom
- Large files (logs): loaded asynchronously; highlighting and wrapping are switched off above 8 MB
- **Developer tools** (header bar and *Developer Tools* context submenu): beautify/minify JSON,
  XML, YAML, SQL; JSON ↔ YAML; MongoDB and PHP array conversions; line tools;
  Base64/URL/JSON-string encode/decode
- **MongoDB / JSON5**: highlighting and auto-detection of mongosh queries and JSON5, beautify/minify
  (pipeline stages one per line), conversion mongosh ↔ Extended JSON v2 (relaxed and canonical)
  and PHP arrays (MongoDB PHP library / Doctrine) ↔ Extended JSON / mongosh, with placeholders
  for PHP variables
- **Checklists** (`- [ ]` / `- [x]`): check items with a click in the editor or the Markdown preview,
  Ctrl+Enter, list continuation on Enter, progress in the status bar
- **Formatting toolbar** for Markdown documents and presentations: headings, bold, italic,
  strikethrough, code, text colours, lists, checklists, quotes, code blocks, rules, tables,
  links, images, charts, formulas and tunes
- **Math formulas** in LaTeX (`$…$`, `$$…$$`, ```` ```math ````), typeset with KaTeX, and
  **music notation** in ABC (```` ```abc ````), engraved with abcjs – in the preview,
  presentations and exports, fully offline
- **Charts from Markdown tables** (```` ```chart ````, drawn with Apache ECharts): line, area,
  bar, pie, donut, rose, scatter, bubble, radar, funnel, gauge, box plot, candlestick, heat map,
  calendar, Sankey, network graph, treemap, sunburst, tree, parallel coordinates and theme
  river; numbers in English or German notation with units and currencies; defaults per
  document in the front matter
- **Flash cards** with spaced repetition (Anki-like scheduling): Obsidian-compatible syntax
  (`Q::A`, `Q:::A`, multi-line cards) plus multiple choice, nested decks, a study window with
  themes, progress and statistics (stored encrypted with the app password, included in backups)
- **Data import**: CSV/TSV files and spreadsheets (Excel, LibreOffice) become Markdown tables;
  cells copied from a spreadsheet can be pasted as table
- **Notes in folders**: a sidebar (F9) with nested folders, drag and drop, a trash; closed notes
  stay in their folder, files can be linked into folders too
- **Optional features** (*Preferences → Features*): charts, presentations, Kanban boards, flash
  cards, note folders, the Geocaching tools, PlantUML and the developer tools can be switched off; they then disappear from menus, toolbar
  and help. New installations start with them off
- **Markdown tables** like Obsidian's *Advanced Tables*: automatic formatting, Tab/Enter
  navigation, insert/delete/move rows and columns, sorting, column alignment
- **Kanban boards**: freely defined columns (title, colour, WIP limit) and swimlanes, drag & drop
  of cards, columns and lanes; stored as readable Markdown (`.kanban.md`)
- Find & replace (case, whole word, regex)
- Word, character and paragraph count in the status bar (of the selection, if any)
- Drag & drop: files open in new tabs, dropped text (e.g. from a browser) is inserted into the note
- 165 colour themes (Dracula, Dayle Rees colour schemes, VSCode Ultimate Themes Pack) besides the
  default dark and light theme; applied instantly to editor, window and preview
- Tabs on top or in a sidebar (vertical tabs), with user-defined colours per tab type
- Distraction-free mode (Alt+Enter): full-screen, centred text without bars or preview
- **Emmet** abbreviations in HTML, XML, XSLT, CSS, SCSS and Less (Tab or Ctrl+E, tab stops for the
  fields, wrap with abbreviation via Ctrl+Shift+G), powered by the official Emmet engine
- Preferences dialog (Ctrl+,) for themes, editor font, language, tabs and import/export
- Font size via Ctrl+mouse wheel or Ctrl +/−/0 (the preview follows)
- **Calculation sheets**: variables, operators, functions and live results per line;
  templates like `> N ab° cd.{e*3:3}` fill in calculated values (e.g. Geocaching coordinates)
- **Mind maps** from Markdown outlines (switchable): branches left/right, to the right or as an
  org chart, themes and branch colours, Markdown styling in the nodes, zoom/fold/jump to the
  line, tick checkboxes and add branches right in the preview, export as SVG or as PDF (A4,
  landscape or portrait to fit)
- **Obsidian vaults**: import a vault with its folders and attachments into the notes library;
  the Markdown preview understands `[[wikilinks]]` (click opens the note), `![[embeds]]`,
  callouts, `==highlights==`, `%%comments%%` and `#tags`
- **Units and conversions** in calculation sheets: values with units (`120 km / 1.5 h`),
  conversions with `to` (`100 °F to °C`, `250 EUR in USD` with daily ECB rates), dates and
  durations (`2026-12-24 - today`), time zones (`14:00 Berlin to Tokyo`), other calendars
  (Julian, Hebrew, Islamic) and a unit converter dialog; unit names are suggested while typing
- **Geocaching tools** in the editor's context menu: ROT13/5/18/47, Caesar, letter value
  (sums, checksums), letter frequency, Scrabble values, prime alphabet, hex, ASCII, binary,
  Roman numerals, Morse code, phone keypad — results are inserted below the selection
- User interface in English or German (*Preferences*, applies after a restart)
- Built-in help (F1) explaining all features, especially calculations and templates
- **App password**: password prompt at start, tabs and notes stored encrypted (Argon2id +
  XChaCha20-Poly1305), lock at any time
- **Tab passwords**: single notes encrypted with their own password, open/closed lock on the tab
- Backup of all notes into a single file and restore (add to or replace the current tabs), optionally
  encrypted with a password (Argon2id + XChaCha20-Poly1305)
- **Sync** (switchable) with GitHub, Nextcloud (browser sign-in), any WebDAV server, a folder synced
  by another program (Dropbox, Syncthing, …) or S3-compatible storage: notes, folders and trash
  end-to-end encrypted with a sync password; at start, every five minutes and when closing;
  both-sides changes kept as conflict copies
- **Data formats** (switchable): JSON, YAML and XML are checked while typing (syntax, duplicate keys,
  yamllint-like style rules) with underlines and a problems list; validation and completion from
  JSON Schema and XML Schema (XSD) – assigned to the note, named in the document or found by file
  name in the SchemaStore catalog
- Editor font selectable in the preferences (e.g. fonts with ligatures like Fira Code); ligatures can be switched off

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| Ctrl+N / Ctrl+T | New note |
| Ctrl+Shift+N | New calculation sheet |
| Ctrl+Shift+K | New Kanban board |
| Ctrl+O | Open file(s) — or drop files onto the window |
| Ctrl+S / Ctrl+Shift+S | Save to file / Save as |
| Ctrl+W | Close tab (notes can be restored via the *Undo* toast) |
| Ctrl+Shift+T | Reopen closed tab |
| F2 | Rename tab |
| F5 | Reload file from disk |
| Ctrl+F / Ctrl+H | Find / Find and replace |
| F3 / Shift+F3 | Next / previous match |
| Ctrl+Alt+B | Beautify (JSON/MongoDB/XML/YAML/SQL, auto-detected) |
| Ctrl+Alt+M | Minify (JSON/MongoDB/XML) |
| Ctrl+Shift+M | Problems list (JSON, YAML, XML) |
| Ctrl+Shift+V | Toggle preview (Markdown, reStructuredText, PlantUML, HTML) |
| Ctrl+Shift+E / Ctrl+Shift+P | Export as HTML / PDF (Markdown, reStructuredText, presentations) |
| Ctrl+F5 / Shift+F5 | Start presentation from the beginning / from the current slide |
| Ctrl+Shift+F5 | Present in a window |
| Ctrl+Enter | Check/uncheck checklist item, turn lines into a checklist |
| Tab, Shift+Tab, Enter | Tables: next/previous cell, next row |
| Shift+Alt+arrow keys | Tables: move row or column |
| Alt+Z | Toggle word wrap |
| Alt+Enter / Esc | Distraction-free mode / leave it |
| Tab, Ctrl+E / Ctrl+Shift+G | Expand Emmet abbreviation / wrap with abbreviation (HTML, CSS, …) |
| Ctrl+, | Preferences |
| Ctrl+wheel, Ctrl +/−, Ctrl+0 | Font size |
| Ctrl+Tab, Ctrl+PgUp/PgDn, Alt+1…9 | Switch tabs |
| F1 | Help |
| Ctrl+Q | Quit |

Developer tools act on the selection, or on the whole tab when nothing is selected.

The complete user documentation is built into the app (F1); its source is
[`data/help.md`](data/help.md).

## Build

The build runs entirely in a Docker container (Ubuntu 24.04, same GTK versions
as the host). The host needs nothing but Docker and the GTK runtime libraries
that ship with an Ubuntu 24.04 desktop.

```sh
./dev.sh                  # cargo build --release  -> target/release/quicknotes
./dev.sh test             # unit tests
./dev.sh clippy           # lints
./dev.sh rebuild-image    # rebuild the dev image (docker/Dockerfile)
./install.sh              # install binary, icon and .desktop entry to ~/.local
```

Runtime libraries used (all part of Ubuntu 24.04): GTK 4.14, libadwaita 1.5,
GtkSourceView 5.12, WebKitGTK 6.0.

## AppImage (other distributions)

```sh
./dev.sh appimage         # -> target/QuickNotes-<version>-x86_64.AppImage
```

The AppImage is self-contained: GTK 4, libadwaita, GtkSourceView, WebKitGTK, a Java
runtime, PlantUML (MIT edition) and Graphviz are bundled, so previews, PDF export and
PlantUML work without installing anything. It was tested on Fedora 42 (the base of
Bazzite) without any of these packages.

What the target system needs – present on every desktop:

- glibc 2.39 or newer (built on Ubuntu 24.04)
- graphics drivers (Mesa: EGL/GLES/GL), X11 and Wayland client libraries, fontconfig
- FUSE to mount the AppImage (otherwise start it with `--appimage-extract-and-run`)

Usage: make it executable (`chmod +x QuickNotes-*.AppImage`) and start it. On Bazzite,
*Gear Lever* adds it to the application menu. Notes are stored in the same place as with
the native build (`~/.local/share/quicknotes`), so both can be used alternately.

WebKit runs web content in a bubblewrap sandbox. If the system cannot provide one (no
`bwrap`/`xdg-dbus-proxy` or no user namespaces), the AppImage starts without it instead
of failing. Build details: `packaging/appimage/`.

## Previews and export

- **Markdown** is rendered with pulldown-cmark (tables, task lists, footnotes, strikethrough,
  math). Formulas are typeset with KaTeX, ABC tunes engraved with abcjs and charts drawn with
  Apache ECharts; all are bundled and only loaded into pages that use them. Exported HTML files
  contain them as well, so they work offline. A YAML front matter is not shown.
- **reStructuredText** is rendered with [docutils-core](https://crates.io/crates/docutils-core),
  a Rust port of docutils – no Python needed. `include` and images resolve relative to the file.
  Problems from level ERROR upwards are shown in the preview.
- **HTML** files are shown as they are (including their CSS and JavaScript). The preview is
  opt-in for HTML (Ctrl+Shift+V) since HTML files are often edited as code.
- **Export** writes a self-contained HTML file, an A4 PDF (via WebKit printing to GTK's
  "Print to File" printer), images (SVG, PNG, JPG – one tall image or one per page/slide,
  converted from that PDF with [hayro](https://github.com/LaurenzV/hayro)) or an EPUB 3 e-book
  (chapters at the top headings, metadata from the front matter). Relative images are resolved
  from the document's folder.

## PlantUML

Rendering uses a local PlantUML installation (nothing is sent to a server). The
first match wins:

1. `plantuml_command` in `~/.local/share/quicknotes/session.json`
2. `$QUICKNOTES_PLANTUML`
3. `plantuml*.jar` in `~/bin`, `~/.local/bin`, `~/.local/share/plantuml`, `/usr/share/plantuml`, `/opt/plantuml` (run with `java -jar`)
4. a `plantuml` executable on `$PATH`

## Backup and restore

*Preferences → Import/Export → Back Up Notes…* writes one `.tar.gz` with all tabs, note contents
(including unsaved changes of opened files) and settings. Opened files themselves
are not included – their tabs only keep the path.

*Preferences → Import/Export → Restore Backup…* offers two modes:

- **Add** – puts the backup's tabs next to the current ones; tabs that are already
  open with the same content are skipped.
- **Replace** – closes all tabs and restores the backup including its settings
  (window size stays). The current state is saved to
  `~/.local/share/quicknotes/backups/before-restore-<time>.tar.gz` first.

Tabs of files that don't exist on this computer are skipped.

## Storage

```
~/.local/share/quicknotes/
├── session.json      # open tabs, settings (font size, theme, window size, …)
├── notes/<id>.txt    # note contents and unsaved changes of opened files
├── language-specs/   # bundled syntax definitions (PlantUML)
├── icons/            # bundled application icon (for the window/About dialog)
├── backups/          # automatic backups taken before "Restore → Replace"
└── github-sync.json  # what was synced last time (encrypted with the app password)

~/.cache/quicknotes/schemas/   # downloaded JSON/XML schemas and the SchemaStore catalog
```

## Data formats

JSON is read by a strict parser of its own (`src/json_lint.rs`), YAML by `yaml-rust2` with
positions (`src/yaml_lint.rs`); both produce a tree that `src/json_schema.rs` validates (drafts 4
to 2020-12, `$ref` across files, `unevaluatedProperties`; `format` is not checked). XML
well-formedness and XSD validation use libxml2 through FFI (`src/xml_lint.rs`), with an entity
loader that serves downloaded schemas from the cache and no network access of its own;
completion uses a simplified model of the XSD (`src/xsd.rs`).

## Sync

*Preferences → Sync* keeps the notes library at one target (folder `quicknotes/`): `vault.json`
holds a random key wrapped with the sync password (Argon2id), `index.enc` the folders and the
notes' metadata, `notes/…enc` each note – all sealed with XChaCha20-Poly1305. Changes are merged
three-way against the last synced state (`src/github.rs`).

- **GitHub**: every sync is one commit via the Git data API (`notes/<id>.enc`).
- **WebDAV / Nextcloud, synced folder, S3** (`src/remote.rs`): no multi-file commits, so note files
  are content-addressed (`notes/<id>-<hash>.enc`) and `index.enc` is written last, with
  `If-Match`/`If-None-Match` where the server supports it; unreferenced files are removed
  afterwards. Every index carries a revision and the previous ones: a machine whose write was
  overwritten notices it and merges without a base, keeping both versions. Nextcloud uses Login
  Flow v2 (app password), S3 requests are signed with AWS Signature V4.

Tokens, passwords and keys are kept in the Secret Service. `cargo test real_servers -- --ignored`
runs the targets against real servers (see the test for the environment variables).

## Notes on X11 drag and drop

GTK 4.14 on X11 has a few drag-and-drop problems with drag sources from other
toolkits; QuickNotes works around them:

- Qt applications (e.g. Dolphin) only hand out drag data to the window that
  received the drop, while GTK 4 requests it from an internal helper window and
  never gets an answer. QuickNotes requests the data itself with the toplevel
  window as requestor (`src/xdnd.rs`).
- After some drag sequences (switching tabs while dragging, Chrome sending
  position updates after the drop) GTK drop targets can get stuck and ignore all
  further drops. QuickNotes replaces its drop targets after every drop.

## License

MIT, see [LICENSE](LICENSE).

## Third-party components

### Emmet

Emmet support uses the official [Emmet](https://github.com/emmetio/emmet) engine (npm package
`emmet` 2.4.11, © Sergey Chikuyonok, MIT License), bundled in `data/emmet/` and run in
JavaScriptCore. It is the engine behind [Emmet for Sublime Text](https://github.com/sergeche/emmet-sublime)
and the other Emmet editor plugins. Thanks to Sergey Chikuyonok and the Emmet contributors.

### KaTeX

Math formulas are typeset with [KaTeX](https://katex.org) (npm package `katex` 0.19.1,
© Khan Academy and other contributors, MIT License), bundled in `data/katex/` with its fonts.

### abcjs

ABC music notation is engraved with [abcjs](https://github.com/paulrosen/abcjs) (npm package
`abcjs` 6.7.2, © Paul Rosen and Gregory Dyke, MIT License), bundled in `data/abcjs/`.

### Apache ECharts

Charts are drawn with [Apache ECharts](https://echarts.apache.org) (npm package `echarts`
6.1.0, © The Apache Software Foundation, Apache License 2.0; parts derived from d3.js under the
BSD 3-Clause License), bundled in `data/echarts/`. The named chart palettes are taken from the
ECharts themes.

### hayro

PDF pages are turned into SVG, PNG and JPG images with [hayro](https://github.com/LaurenzV/hayro)
and hayro-svg (© The Hayro Authors, MIT/Apache-2.0); JPG files are written with
[jpeg-encoder](https://github.com/vstroebel/jpeg-encoder) (© Volker Ströbel, MIT/Apache-2.0).

### calamine and csv

Spreadsheets are read with [calamine](https://github.com/tafia/calamine) (MIT License) and
CSV files with [csv](https://github.com/BurntSushi/rust-csv) (MIT/Unlicense).

### Colour themes

The bundled colour themes are converted from these MIT-licensed projects:

- [Dracula](https://github.com/dracula/dracula-theme) – Dracula and Alucard
  (© Dracula Theme)
- [Colour Schemes](https://github.com/daylerees/colour-schemes) by Dayle Rees – 57 themes
  (© Dayle Rees)
- [VSCode Ultimate Themes Pack](https://github.com/CodewithEvilxd/VSCode-Ultimate-Themes-Pack)
  by codewithevilxd – 106 themes (© codewithevilxd)

Many thanks to their authors. The full license texts are in
[THIRD_PARTY_THEMES.md](THIRD_PARTY_THEMES.md); the copyright notices are also shown in
*About QuickNotes → Legal*. The themes are converted with
`tools/themes/convert.py <folder with the cloned repositories>`.
