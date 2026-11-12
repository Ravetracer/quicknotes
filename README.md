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
- Export of Markdown and reStructuredText documents to **HTML** and **PDF**
- **Presentations** in Markdown: slides separated by `---`, slide transitions, step-by-step
  reveal (`::: steps fade-up`), positioning (`::: place bottom right`, columns), background
  images, presenter notes (`???`); presenter view with notes, next slide and timer on a second
  screen; export as stand-alone HTML or PDF
- Image viewer (PNG, JPEG, GIF, WebP, SVG, BMP, ICO, TIFF, …) with zoom
- Large files (logs): loaded asynchronously; highlighting and wrapping are switched off above 8 MB
- Beautify/minify JSON, XML, YAML, SQL; JSON ↔ YAML; line tools; Base64/URL/JSON-string encode/decode
- **MongoDB / JSON5**: highlighting and auto-detection of mongosh queries and JSON5, beautify/minify
  (pipeline stages one per line), conversion mongosh ↔ Extended JSON v2 (relaxed and canonical)
  and PHP arrays (MongoDB PHP library / Doctrine) ↔ Extended JSON / mongosh, with placeholders
  for PHP variables
- **Checklists** (`- [ ]` / `- [x]`): check items with a click in the editor or the Markdown preview,
  Ctrl+Enter, list continuation on Enter, progress in the status bar
- **Formatting toolbar** for Markdown documents and presentations: headings, bold, italic,
  strikethrough, code, text colours, lists, checklists, quotes, code blocks, rules, tables,
  links, images, formulas and tunes
- **Math formulas** in LaTeX (`$…$`, `$$…$$`, ```` ```math ````), typeset with KaTeX, and
  **music notation** in ABC (```` ```abc ````), engraved with abcjs – in the preview,
  presentations and exports, fully offline
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

Tools act on the selection, or on the whole tab when nothing is selected.

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
  math). Formulas are typeset with KaTeX and ABC tunes engraved with abcjs; both are bundled
  and only loaded into pages that use them. Exported HTML files contain them as well, so they
  work offline.
- **reStructuredText** is rendered with [docutils-core](https://crates.io/crates/docutils-core),
  a Rust port of docutils – no Python needed. `include` and images resolve relative to the file.
  Problems from level ERROR upwards are shown in the preview.
- **HTML** files are shown as they are (including their CSS and JavaScript). The preview is
  opt-in for HTML (Ctrl+Shift+V) since HTML files are often edited as code.
- **Export** writes a self-contained HTML file or an A4 PDF (via WebKit printing to GTK's
  "Print to File" printer). Relative images are resolved from the document's folder.

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
└── backups/          # automatic backups taken before "Restore → Replace"
```

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
