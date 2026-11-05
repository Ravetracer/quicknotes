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
- Live preview side-by-side for **Markdown** and **PlantUML**
- Image viewer (PNG, JPEG, GIF, WebP, SVG, BMP, ICO, TIFF, …) with zoom
- Large files (logs): loaded asynchronously; highlighting and wrapping are switched off above 8 MB
- Beautify/minify JSON, XML, YAML, SQL; JSON ↔ YAML; line tools; Base64/URL/JSON-string encode/decode
- Find & replace (case, whole word, regex)
- Drag & drop: files open in new tabs, dropped text (e.g. from a browser) is inserted into the note
- Dark and light theme
- Font size via Ctrl+mouse wheel or Ctrl +/−/0 (the preview follows)

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| Ctrl+N / Ctrl+T | New note |
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
| Ctrl+Shift+V | Toggle preview (Markdown, PlantUML) |
| Alt+Z | Toggle word wrap |
| Ctrl+wheel, Ctrl +/−, Ctrl+0 | Font size |
| Ctrl+Tab, Ctrl+PgUp/PgDn, Alt+1…9 | Switch tabs |
| Ctrl+Q | Quit |

Tools act on the selection, or on the whole tab when nothing is selected.

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

## PlantUML

Rendering uses a local PlantUML installation (nothing is sent to a server). The
first match wins:

1. `plantuml_command` in `~/.local/share/quicknotes/session.json`
2. `$QUICKNOTES_PLANTUML`
3. `plantuml*.jar` in `~/bin`, `~/.local/bin`, `~/.local/share/plantuml`, `/usr/share/plantuml`, `/opt/plantuml` (run with `java -jar`)
4. a `plantuml` executable on `$PATH`

## Storage

```
~/.local/share/quicknotes/
├── session.json      # open tabs, settings (font size, theme, window size, …)
├── notes/<id>.txt    # note contents and unsaved changes of opened files
├── language-specs/   # bundled syntax definitions (PlantUML)
└── icons/            # bundled application icon (for the window/About dialog)
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
