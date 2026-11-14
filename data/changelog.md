## 0.18.0 – 2026-11-14

- **More sync targets**: besides GitHub, notes sync with Nextcloud (sign-in in the browser), any WebDAV server, S3-compatible storage or a folder kept in sync by another program (Dropbox, Syncthing, …) – end-to-end encrypted, at start, every five minutes and when closing.
- **Data formats** (switchable): JSON, YAML and XML are checked while you type – underlines, tooltips and a problems list (Ctrl+Shift+M); yamllint-like style rules for YAML.
- **Schemas**: validation and completion from JSON Schema and XML Schema (XSD) – assigned to a note, named in the document or found by file name in the SchemaStore catalog.
- **Note templates**: notes in a *Templates* folder with a Twig-like syntax – date and time fields, questions asked on insert, cursor, selection and clipboard, building blocks (`include`) and inheritance (`extends`/`block`). New notes from templates (Ctrl+Alt+N) stay linked: later template changes are merged in, your own edits are kept.
- **GM tools** (switchable): adventures for game masters with chapters, character sheets (presets for D&D 5e, Pathfinder 2e, DSA 5), a dice tray (keep/drop, exploding, success pools, Fate, DSA checks, custom dice), initiative, play and world time and a session log.
- **What's new** after an update, the full changelog in *About QuickNotes* and a check for new versions at start (can be switched off in the preferences).
- Fixed: JSON files with a `$schema` opened as JavaScript; a crash when a new note started with certain umlauts.

## 0.17.0 – 2026-11-14

- **Mind maps** (switchable) from Markdown outlines: Markdown in the nodes, three layouts, themes, colours and folded branches per node.
- Interactive preview: zoom, pan, fold, tick checkboxes, add branches, double-click to jump to the line.
- Export as SVG or PDF.

## 0.16.0 – 2026-11-14

- **Import Obsidian vaults** into a library folder, with attachments; re-importing adds only new notes.
- Obsidian syntax in every Markdown preview: wikilinks, embeds, highlights, comments, tags and callouts.

## 0.15.0 – 2026-11-14

- **Units in calculation sheets**: values with units, conversions (`to`, `in`, `as`), your own units, number systems and Roman numerals.
- **Currencies** with the daily rates of the European Central Bank.
- **Dates, durations, time zones and calendars**, including Julian, Hebrew and Islamic calendars.
- Unit converter dialog and completion of unit names.

## 0.14.0 – 2026-11-14

- **Encrypted sync with GitHub**: notes, folders and trash, end-to-end encrypted with a sync password; changes on both sides are kept as conflict copies.

## 0.13.0 – 2026-11-13

- **Notes library** with nested folders in a sidebar (F9): closed notes stay in their folder, plus a trash.

## 0.12.0 – 2026-11-13

- **Export as image** (PNG, JPEG, WebP) and **EPUB**.
- **Flash cards** with spaced repetition and a learning overview.
- Context help: F1 opens the help at the topic you are working on.

## 0.11.0 – 2026-11-13

- **Switchable features** in the preferences, including a Developer Tools menu.
- Fixed: the height of context menus.

## 0.10.0 – 2026-11-13

- **Charts from Markdown tables** (bar, line, pie and more).
- Import CSV, Excel and ODS data as tables; paste as table.

## 0.9.0 – 2026-11-12

- **Markdown formatting toolbar** and context-aware menus.
- **LaTeX math** (KaTeX) and **ABC music notation** in Markdown.

## 0.8.0 – 2026-11-12

- **Checklists**, **table editing** and **Kanban boards**.
- Word, character and paragraph count in the status bar.
- **App password**, **tab passwords** and password-protected backups.

## 0.7.0 – 2026-11-09

- **Emmet** abbreviations for HTML, XML and CSS.

## 0.6.0 – 2026-11-09

- **Preferences dialog**, vertical tabs, distraction-free mode and 165 colour themes.
- The editor/preview split is kept per tab.

## 0.5.0 – 2026-11-06

- **MongoDB / JSON5** support and conversion between PHP arrays and Extended JSON.

## 0.4.0 – 2026-11-06

- Contextual menus, **tab colours** and syntax help for presentations.

## 0.3.0 – 2026-11-06

- **Markdown presentations** with a presenter view.

## 0.2.0 – 2026-11-05

- **Geocaching tools** (ciphers, coordinates, puzzle helpers).
- German user interface.

## 0.1.0 – 2026-11-05

- First version: a tabbed notepad with autosave and syntax highlighting.
- Markdown, reStructuredText, HTML and PlantUML preview; export as HTML and PDF.
- **Calculation sheets** with templates, backup and restore, in-app help and an AppImage.
