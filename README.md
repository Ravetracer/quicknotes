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
- Find & replace (case, whole word, regex)
- Drag & drop: files open in new tabs, dropped text (e.g. from a browser) is inserted into the note
- Dark and light theme
- Font size via Ctrl+mouse wheel or Ctrl +/−/0 (the preview follows)
- **Calculation sheets**: variables, operators, functions and live results per line;
  templates like `> N ab° cd.{e*3:3}` fill in calculated values (e.g. Geocaching coordinates)
- **Geocaching tools** in the editor's context menu: ROT13/5/18/47, Caesar, letter value
  (sums, checksums), letter frequency, Scrabble values, prime alphabet, hex, ASCII, binary,
  Roman numerals, Morse code, phone keypad — results are inserted below the selection
- User interface in English or German (main menu → *Language*, applies after a restart)
- Built-in help (F1) explaining all features, especially calculations and templates
- Backup of all notes into a single `.tar.gz` and restore (add to or replace the current tabs)
- Editor font selectable in the status bar (e.g. fonts with ligatures like Fira Code); ligatures can be switched off in the main menu

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| Ctrl+N / Ctrl+T | New note |
| Ctrl+Shift+N | New calculation sheet |
| Ctrl+O | Open file(s) — or drop files onto the window |
| Ctrl+S / Ctrl+Shift+S | Save to file / Save as |
| Ctrl+W | Close tab (notes can be restored via the *Undo* toast) |
| Ctrl+Shift+T | Reopen closed tab |
| F2 | Rename tab |
| F5 | Reload file from disk |
| Ctrl+F / Ctrl+H | Find / Find and replace |
| F3 / Shift+F3 | Next / previous match |
| Ctrl+Alt+B | Beautify (JSON/XML/YAML/SQL, auto-detected) |
| Ctrl+Alt+M | Minify (JSON/XML) |
| Ctrl+Shift+V | Toggle preview (Markdown, reStructuredText, PlantUML, HTML) |
| Ctrl+Shift+E / Ctrl+Shift+P | Export as HTML / PDF (Markdown, reStructuredText, presentations) |
| Ctrl+F5 / Shift+F5 | Start presentation from the beginning / from the current slide |
| Ctrl+Shift+F5 | Present in a window |
| Alt+Z | Toggle word wrap |
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

- **Markdown** is rendered with pulldown-cmark (tables, task lists, footnotes, strikethrough).
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

*Main menu → Back Up Notes…* writes one `.tar.gz` with all tabs, note contents
(including unsaved changes of opened files) and settings. Opened files themselves
are not included – their tabs only keep the path.

*Main menu → Restore Backup…* offers two modes:

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
