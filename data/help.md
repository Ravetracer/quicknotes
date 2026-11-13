# QuickNotes Help {#top}

**Contents:** [Notes & tabs](#notes) · [Folders](#folders) · [Checklists](#checklists) · [Tables](#tables) · [Formatting](#formatting) · [Math & music](#notation) · [Charts](#charts) · [Kanban](#kanban) · [Flash cards](#flashcards) · [Calculations](#calc) · [Templates](#templates) ·
[Previews & export](#previews) · [Presentations](#slides) · [Developer tools](#devtools) · [Emmet](#emmet) · [Geocaching](#geocaching) ·
[Security](#security) · [Backup](#backup) ·
[Shortcuts](#shortcuts)

## Notes & tabs {#notes}

Every tab is either a **note** or an **opened file**.

- **Notes** are saved automatically – there is no need to save them as a file. When you
  close QuickNotes and start it again, all tabs are back, including the cursor position.
- **Opened files** behave like in any editor: *Save* (Ctrl+S) writes them back. Unsaved
  changes are kept by QuickNotes until you save or discard them, even across restarts.
- A closed note can be restored with the **Undo** button or *Reopen Closed Tab*
  (Ctrl+Shift+T).
- The tab title is the first line of a note. *Rename Tab* (F2) sets a fixed title.
- Syntax highlighting is chosen automatically and can be changed in the status bar.
- The status bar counts words, characters (without line breaks) and paragraphs of the tab, or
  of the selection when text is selected.
- Tabs are tinted by type: code, documents (Markdown, reStructuredText), presentations,
  calculations and images each have their own colour (adjustable in the preferences); plain
  notes keep the default look.
- Tabs are shown on top or in a sidebar on the left (*Preferences → Tabs*). The sidebar width
  can be dragged; tabs can be reordered by drag and drop and closed with a middle click.
- **Distraction-free mode** (Alt+Enter, Esc to leave) shows only the text, full screen and
  centred, without bars, tabs, preview or line numbers.

### Preferences

*Main menu → Preferences* (Ctrl+,) sets the editor font, its size and ligatures, the
interface language (applies after a restart), the colour theme, the tab position and the tab
colours, and offers backup and restore of the notes (*Import/Export*).

*Features* switches optional parts of QuickNotes on and off: charts, presentations, Kanban
boards, flash cards, note folders, the Geocaching tools, PlantUML and the developer tools. Switched-off features disappear from the menus, the
toolbar and this help; documents stay as they are (presentations and boards then open as
Markdown, charts show as code). New installations start with all of them off.

*Themes* lists the default dark and light theme and 165 themes from Dracula, the Dayle Rees
colour schemes and the VSCode Ultimate Themes Pack. A click applies a theme right away to the
editor, the window and the preview; Ctrl+F in the preferences searches themes by name. The font size can also be changed with
Ctrl+mouse wheel or Ctrl +/−/0.

## Folders {#folders}

The **notes sidebar** (button at the top left, or F9) keeps all your notes in folders – also
the ones that are not open. Closing a note's tab no longer throws it away: the note stays in
its folder and opens again with a click. Open notes are shown in bold.

- **New Folder**: the folder button above the tree, or *New Folder* in a folder's right-click
  menu for a sub-folder. Folders can be nested as deep as you like and are sorted by name.
- **Moving**: drag a note onto a folder (or onto *Ungrouped*), drag a folder into another
  folder. *Move to Ungrouped* in the right-click menu takes a note out of its folder.
- **New Note** in a folder's right-click menu creates the note right there.
- **Files** opened from disk can be put into folders as well. After closing they stay there as
  a link that opens the file again; a warning sign marks a file that no longer exists.
  *Remove from Notes* takes the link away (the file itself stays).
- **Deleting**: *Move to Trash* (or dragging onto *Trash*) puts a note into the trash; from there
  it can be restored or deleted permanently (*Empty Trash* deletes all of it). Deleting a
  folder keeps its notes and sub-folders – they move one level up.
- Empty notes without a title are still discarded when closed.

Folders, closed notes and the trash are encrypted with the app password and included in
backups.

## Checklists {#checklists}

Notes, Markdown documents and presentations understand task lists in the common Markdown
format, so they stay readable everywhere (GitHub, Obsidian, …):

```
- [ ] open item
- [x] done item
```

- **Click the box** `[ ]` in the editor or the checkbox in the Markdown preview to check or
  uncheck an item. Done items are struck through.
- **Ctrl+Enter** checks the item on the cursor line (or all selected items; unchecks them when
  all are done). On lines that are no items yet it turns them into items – select a few lines
  of ideas and press Ctrl+Enter to get a checklist.
- **Enter** at the end of an item starts the next, unchecked item; Enter on an empty item ends
  the list. This also works for normal bullet and numbered lists.
- The status bar shows the progress, e.g. *✓ 3 of 7 done* (of the selection, if any).
  Items in code blocks are not counted.

## Tables {#tables}

Markdown tables (in notes, Markdown and presentations) are edited like in Obsidian's
*Advanced Tables*: the table is formatted automatically while you move through it, so the
columns always line up – also with umlauts and wide characters.

- Start a table by typing `| Name` and pressing **Tab**: the header gets a new column and the
  separator line is added. Or use *Insert Table* in the context menu.
- **Tab** / **Shift+Tab** move to the next / previous cell and select its text, so typing
  replaces it. Tab in the last cell of a row adds a new column.
- **Enter** moves to the same column in the next row and adds a row at the end. Enter in an
  empty last row removes it and leaves the table.
- Inside a table the context menu (right click into a cell, or Shift+F10) shows only the table
  commands: format, insert, move and delete rows and columns, **sort** the rows by the column
  (numbers numerically, text alphabetically) and **align** the column left, centred or right
  (`:--`, `:-:`, `--:`).
- **Shift+Alt+↑/↓** move the row, **Shift+Alt+←/→** move the column.
- *Beautify* (Ctrl+Alt+B) in a Markdown document formats all its tables at once.
- **Import data** (table menu in the toolbar, or the context menu): a CSV/TSV file or a
  spreadsheet (Excel `.xlsx`/`.xls`, LibreOffice `.ods`) becomes a Markdown table. You can pick
  the sheet, the separator (detected automatically; Excel's `;` included) and whether the first
  row is the header. Dates from spreadsheets become `YYYY-MM-DD`.
- **Paste as Table** turns cells copied from a spreadsheet (or other tab/comma separated
  text) into a table.

## Formatting {#formatting}

Markdown documents and presentations have a **formatting toolbar** above the text. It writes
the Markdown for you, so you don't need to remember the syntax:

- **Headings** 1–6 (and back to normal text), **bold** (Ctrl+B), *italic* (Ctrl+I),
  ~~strikethrough~~ and `inline code`. Without a selection they apply to the word at the
  cursor. Applying a style again removes it.
- **Text colour**: wraps the selection in `<span style="color: …">`, shown in the preview and
  in HTML/PDF exports. *Remove Colour* takes it off again.
- Bulleted and numbered lists, checklists and quotes for the selected lines (again to remove),
  code blocks, horizontal rules and tables.
- **Link** (Ctrl+K) and **image**: the selection becomes the text, a selected URL the target.
<!-- if charts -->
- **Chart** (a chart from the table above, with sample data if there is none), see
  [Charts](#charts).
<!-- end -->
- **∑ Formula** (inline or as a block) and **♫ Music notation** (an ABC tune), see below.

*Main menu → Formatting Toolbar* hides the toolbar; the shortcuts keep working.

## Math & music {#notation}

Markdown documents and presentations can contain **LaTeX formulas** and **music notation**.
Both are shown in the preview, in presentations and in HTML/PDF exports (no internet needed).

**Formulas** use LaTeX math syntax (typeset with KaTeX):

- `$E = mc^2$` – a formula in the text. A `$` with a space after it opens no formula and one
  with a space before it closes none, so prices like $5 or $10 stay as they are.
- `$$ \int_0^1 x^2\,dx $$` – a formula on its own, centred.
- A code block with the language `math` – handy for longer formulas over several lines:

      ```math
      \begin{pmatrix} a & b \\ c & d \end{pmatrix}
      ```

Fractions `\frac{a}{b}`, roots `\sqrt{x}`, sums `\sum_{k=1}^n`, integrals, matrices, Greek
letters `\alpha`, … are supported; a mistake shows the formula in red instead of breaking the page.

**Music** is written in [ABC notation](https://abcnotation.com) in a code block with the
language `abc` (engraved with abcjs):

      ```abc
      X:1
      T:Scale
      M:4/4
      L:1/4
      K:D
      DEFG|ABcd|
      ```

The header lines set index (`X`), title (`T`), metre (`M`), default note length (`L`) and key
(`K`, always last); then the melody follows: `C D E` are notes, `c d e` an octave higher,
`C2` twice as long, `|` a bar line, `^` / `_` sharp / flat, `z` a rest.

## Charts {#charts}

Markdown documents and presentations can draw **charts from their tables** (with Apache
ECharts, shown in the preview, presentations and HTML/PDF exports, no internet needed). A code
block with the language `chart` holds the settings as `key: value` lines; the data comes from
a table of the document – by default the nearest one **above** the block:

    | Month | Revenue | Costs |
    |-------|---------|-------|
    | Jan   | 1.200 € | 800 € |
    | Feb   | 1.500 € | 950 € |

      ```chart
      type: bar
      title: Revenue and costs
      ```

The **chart button** in the toolbar inserts such a block (with a sample table when there is no
table above). Mistakes are shown in red in place of the chart.

**Which table:** `table: above` (default; without a table above, the one below is taken),
`table: below`, `table: 2` (the second table of the document) or `table: Sales` (the first
table under the heading "Sales").

**Numbers in cells** may be written as you like: `1.234,56` and `1,234.56`, `12 %`, `€ 5`,
`5 kg`, `(120)` for −120. Empty cells, `-` and `n/a` are gaps. If a number could be read either
way (`1.234`), the other numbers of the column decide, then `decimal: ","` or `"."`, then the
app language. A unit all numbers share appears on the axis. Dates can be `2026-03-01`,
`01.03.2026` or `2026-03`.

**Chart types** and the columns they use (columns are named by header or number; without
settings the first column gives the names/categories and all number columns the values):

| Type | Data |
|------|------|
| `line`, `area`, `bar` | `x` column (categories, dates give a time axis) and `y` columns; `group:` makes series of a column's values (long tables) |
| `pie`, `donut`, `rose` | names and a value column (several value columns: several pies) |
| `scatter`, `bubble` | `x` and `y` numbers; `size` (bubbles), `label`, `group` |
| `radar` | the rows are the axes, every value column a shape |
| `funnel` | names and values |
| `gauge` | a gauge per row – or a fixed value: `value: 72` (`min`, `max`, `unit`) |
| `boxplot` | a box per number column, or `x: Group` with a value column |
| `candlestick` | date, open, high, low, close (found by header or in this order) |
| `heatmap` | a matrix (row names, a column per x value) – or `x`, `y` and `value` columns |
| `calendar` | a date and a value column; one calendar per year |
| `sankey`, `graph` | `source`, `target` and `value` columns (`directed: true`, `layout: circular` for graphs) |
| `treemap`, `sunburst`, `tree` | path columns (Region / Country / City) and a value – or `name` and `parent` columns |
| `parallel` | an axis per column (`y: [...]` to choose), `group` for colours |
| `themeriver` | dates/numbers and value columns (or `group` and `value`) |

**Settings:** `title`, `subtitle`, `stack: true`, `horizontal: true`, `smooth: true`,
`step: true`, `labels: true` (show values), `legend: top|bottom|left|right|false`,
`zoom: true`, `sort: asc|desc`, `y2: Column` (second axis on the right, as line),
`min`, `max`, `log: true`, `unit: "€"`, `height: 400`, `theme: light|dark` (default: like the
page), `palette:` a name (`default`, `gnome`, `macarons`, `vintage`, `roma`, `shine`,
`infographic`, `royal`, `sakura`, `caravan`, `azul`, `helianthus`, `blue`, `green`, `red`,
`gray`) or a list of colours, `animation: true`. Anything else ECharts offers goes below
`echarts:` and is merged into the chart, e.g. `echarts: { yAxis: [{ splitLine: { show: false } }] }`.

**Defaults for all charts** of a document go into the front matter:

    ---
    charts:
      palette: vintage
      height: 300
      decimal: ","
    ---

## Flash cards {#flashcards}

**Flash cards** help you learn with spaced repetition: each card comes back when you are about
to forget it. *Main menu → New Flash Cards* starts a document with examples. A Markdown
document is a set of flash cards when its front matter says `flashcards: true` (or the file
ends in `.cards.md`).

    ---
    flashcards: true
    deck: Biology/Cells
    ---

    # Basics

    Capital of France::Paris
    Mitochondria:::Powerhouse of the cell

    What is osmosis?
    ?
    Water moves through a membrane
    from low to high concentration.

    Which are organelles?
    ?
    - [x] Mitochondrion
    - [x] Ribosome
    - [ ] Cell wall
    Ribosomes have no membrane.

    The largest planet?
    ?
    - ( ) Mars
    - (x) Jupiter

- `Question::Answer` – a card on one line.
- `Question:::Answer` – two cards: the question is also asked the other way round.
- Question lines, a line with only `?`, then the answer lines – a card over several lines
  (`??` for both ways). A blank line ends the card. Answers can use all of Markdown: lists,
  code, formulas, images.
- An answer of `- [x]` / `- [ ]` items is **multiple choice with check boxes** (any number of
  right answers), `- (x)` / `- ( )` items is **multiple choice with radio buttons** (exactly
  one right answer). Lines after the items explain the answer.

**Decks:** `deck: Biology/Cells` in the front matter puts the cards into a deck and its
sub-deck (as deep as you like; several documents can feed the same deck). Headings in the
document form further sub-decks. Without `deck:` the title (or the file name) is the deck.

**Learning:** the preview shows the decks with their progress and statistics. **Learn** opens
the cards that are due and new ones (at most 20 new cards a day, `new-per-day:` changes it);
**Practice** goes through all cards of a deck in random order without changing the schedule.
In the study window:

| Key | Action |
|---|---|
| Space, Enter, click | Show the answer (multiple choice: check the selection) |
| 1–9 | Multiple choice: select an answer |
| 1 – 4 | Again, Hard, Good, Easy (Space: Good) |
| F11 | Full screen |
| Esc | End |

Each button tells when the card comes back. *Again* repeats the card in the same session.
Wrong answers can be practised once more at the end. *Main menu → Learning Overview* shows
the decks of all open flash card documents together.

**Progress** is stored in QuickNotes' data folder, not in the document – encrypted while the
app password is on, and included in backups. A card keeps its progress when its question is
changed a little. Statistics count the answers, the right ones, the time spent learning and the
days in a row.

**Appearance:** `theme:` in the front matter chooses `light`, `dark`, `sepia`, `night` or
`contrast` (default: like QuickNotes). `accent:`, `background:` and `color:` set colours
(`#rrggbb`); text colours are adjusted automatically if they would be hard to read.

## Kanban boards {#kanban}

*Main menu → New Kanban Board* (Ctrl+Shift+K) creates a board with columns (e.g. *To do*,
*Doing*, *Done*) and optional **swimlanes** (rows across all columns, e.g. per project or person).

- **Cards:** *+ Add card* in a cell, click a card to edit it – the first line is the title,
  further lines (Shift+Enter) the description; Enter saves, Esc cancels. The checkbox marks a
  card as done, × deletes it. Drag cards with the mouse to another place, column or swimlane.
- **Columns:** *+ Column* adds one. Drag the column header to reorder; the ⋯ menu renames it
  (or double-click the header), sets its **colour** and a **WIP limit** (the counter turns red
  when the column holds more cards), moves or deletes it.
- **Swimlanes:** *+ Swimlane* adds one (the first lane gets a name, too). Drag the lane header to
  reorder, ▾ collapses it, the ⋯ menu renames, moves or deletes it.
- Click the board title to rename it. Ctrl+Z / Ctrl+Shift+Z undo and redo board changes.
- The status bar counts the cards and the done ones.

A board is stored as readable Markdown (lanes as `#` headings, columns as `##` headings, cards
as `- [ ]` items, colours and limits in the front matter), so it stays usable in any editor and
on the phone. *Preview* (Ctrl+Shift+V) shows this source next to the board; changes to it
update the board. Saved as a file, boards get the extension `.kanban.md`. The board view
rewrites the source in its own layout – free text between the cards becomes part of the card
above it.

## Calculations {#calc}

A **calculation sheet** evaluates every line and shows the result at the right edge of the
line. Create one with *New Calculation* (Ctrl+Shift+N) or select **Calculation** as syntax
in the status bar for any tab. Results update while you type; **click a result to copy it**.

```
a = 5            # define variables
b = 10
d = a + b        # = 15
d * 2            # = 30
(a + b) / 4      # = 3.75
```

**Rules**

- `name = expression` defines a variable. Names start with a letter and may contain
  letters, digits and `_`. Upper and lower case are different variables (`a` ≠ `A`).
- Lines are evaluated from top to bottom; a variable can be redefined later on.
- A line that is only an expression shows its value. Plain assignments like `a = 5` show
  nothing, because the value is already there.
- `#` or `//` start a comment. Lines that are not a calculation (normal text) are ignored,
  so you can write notes between your calculations.
- Errors are shown in red, e.g. *unknown variable "x"* or *division by zero*.

**Operators**

| Operator | Meaning | Example |
|---|---|---|
| `+` `-` | addition, subtraction | `a + b - 1` |
| `*` `×` `·` | multiplication | `a * 3` |
| `/` `÷` `:` | division | `a / 4` |
| `%` | remainder (modulo) | `17 % 5` = 2 |
| `^` `**` | power | `2 ^ 10` = 1024 |
| `!` | factorial | `5!` = 120 |
| `( )` `[ ]` | brackets | `(a + b) * [c - 1]` |

A number or closing bracket directly before a name or bracket multiplies:
`2a` = `2 * a`, `3(a + b)` = `3 * (a + b)`. The decimal separator is the point (`1.5`).

**Functions**

| Function | Description |
|---|---|
| `sqrt(x)` | square root |
| `abs(x)` | absolute value |
| `round(x)`, `round(x, n)` | round, optionally to *n* decimals |
| `floor(x)`, `ceil(x)`, `trunc(x)` / `int(x)` | round down, up, cut off decimals |
| `min(…)`, `max(…)`, `sum(…)`, `avg(…)` | of any number of values |
| `qs(x)` / `digitsum(x)` | digit sum (*Quersumme*): `qs(1234)` = 10 |
| `iqs(x)` / `digitroot(x)` | iterated digit sum down to one digit: `iqs(98765)` = 8 |
| `sin`, `cos`, `tan`, `asin`, `acos`, `atan` | trigonometry in **radians** |
| `rad(x)`, `deg(x)` | degrees → radians, radians → degrees: `sin(rad(30))` = 0.5 |
| `ln(x)`, `log(x)`, `log2(x)`, `exp(x)` | logarithms (e, 10, 2) and e^x |
| `pi` | 3.14159… (a variable named `pi` takes precedence) |

## Templates {#templates}

A line starting with `>` is a **template**. Its filled-in text appears as result and can be
copied with a click – ideal for coordinates in Geocaching, but useful for anything that
combines calculated values into a text.

**Letters as placeholders** – every single letter that is a defined variable is replaced by
its value; all other characters stay as they are:

```
a = 5
b = 2
c = 1
d = 2
e = 3
f = 4
g = 5
> Nab° cd.efg        → N52° 12.345
```

`N` stays `N` here because no variable `N` exists. To keep a letter even though such a
variable exists, put a backslash in front of it: `\N`.

**Calculations in braces** – `{…}` contains any expression:

```
a = 7
b = 3
> N 52° {a*b}.{a*100+b} E 013° {b*4}.{(a+b)*12}
                     → N 52° 21.703 E 013° 12.120
```

**Formatting** – after a colon inside the braces:

| Format | Meaning | Example (x = 7.256) |
|---|---|---|
| `{x:3}` | at least 3 digits before the point, filled with zeros | `007.256` |
| `{x:.2}` | exactly 2 decimals (rounded) | `7.26` |
| `{x:2.1}` | both | `07.3` |

Minutes in coordinates usually need two digits and three decimals, e.g.
`> N 52° {m:2}.{t:3}`. Inside braces, use `/` for division (the colon starts the format).

## Previews & export {#previews}

<!-- if plantuml -->
Markdown, reStructuredText, PlantUML and HTML files can be shown with a live preview next to
<!-- end -->
<!-- if !plantuml -->
Markdown, reStructuredText and HTML files can be shown with a live preview next to
<!-- end -->
the text (Ctrl+Shift+V). Markdown and reStructuredText can be exported from the main menu or
the editor's right-click menu as:

- **HTML** (Ctrl+Shift+E): a single file with everything in it, also formulas and charts.
- **PDF** (Ctrl+Shift+P): A4 pages.
- **Image** (SVG, PNG or JPG): the whole document as **one tall image**, or **one image per
  page** like the PDF (numbered files in a folder you choose). SVG stays sharp at any size; PNG
  and JPG are pictures in high resolution.
- **EPUB**: an e-book for e-readers and reading apps. A new chapter starts at each `#` heading
  (at each `##` heading if there is only one `#`); the table of contents lists the chapters and
  their sections. Formulas, charts and music notation are included, as are the images of the
  document. The front matter can set the book's details:

      ---
      title: My Book
      author: Jane Doe
      language: en
      date: 2026-11-13
      description: What the book is about
      publisher: Self-published
      cover: images/cover.jpg
      ---

Relative images are taken from the folder of the document.

<!-- if plantuml -->
PlantUML needs a local PlantUML installation (`plantuml` command or a `plantuml*.jar` in
`~/bin`).
<!-- end -->

## Presentations {#slides}

A **presentation** is Markdown split into slides. Create one with *Main menu → New
Presentation* – it starts with a short example. Files named `*.slides.md`, and Markdown files
with `presentation: true` in their front matter, open as presentations; any Markdown tab can
be switched to *Presentation* in the status bar.

The preview shows the slide at the cursor together with its notes; **‹ ›** jump to the
previous/next slide in the text, **►** plays the slide's animations.

While typing, QuickNotes suggests what fits at the cursor – after `::: `, in the front matter,
in `<!-- … -->` settings and in image options `![…]` – each with a short explanation
(Ctrl+Space shows the list on demand). **Shift+F1** (or *Syntax Help* in the right-click menu)
opens a cheat sheet of all options for the cursor position.

### Slides and settings

A line with only `---` starts a new slide (inside code blocks it stays code). Settings for the
whole presentation go into a front matter block at the very top:

```
---
presentation: true
title: My Talk
theme: default        # default, dark, ocean, sunset
aspect: 16:9          # 16:9, 4:3, 16:10
transition: fade 500ms
step: fade-up 400ms   # default animation of steps
paginate: true        # slide numbers
header: Text at the top
footer: Text at the bottom
duration: 20m         # talk length, shown as remaining time
style: |
  .qn-slide h1 { text-transform: uppercase; }
---
```

Settings of a single slide are written as HTML comment anywhere on the slide, several of
them separated by `;`:

```
<!-- transition: zoom 0.8s; background: #1e3a5f; color: white; class: lead -->
```

Slide settings: `transition`, `step`, `theme`, `background` (color, gradient),
`color`, `class`, `align` (left, center, right), `valign` (top, middle, bottom), `header`,
`footer`, `paginate`. Classes: `lead` (centered title slide), `invert` (swapped colors).

### Animations

Durations are written as `400ms` or `0.8s`, or as speed `fast`, `normal`, `slow`.

- **Slide transitions** (`transition:`): `none`, `fade`, `slide-left` (`slide`),
  `slide-right`, `slide-up`, `slide-down`, `cover`, `zoom`, `zoom-out`, `flip`, `cube`,
  `rotate`, `blur`. Going back plays the transition in reverse.
- **Step animations**: `fade`, `fade-up`, `fade-down`, `fade-left`, `fade-right`, `zoom`,
  `zoom-out`, `flip`, `rotate`, `blur`, `bounce`, `drop`, `wipe`, `none` (just appear).

### Step by step

Wrap content in a **container** – a line `:::` with words, closed by a line `:::` –
to show it on click or key press:

```
::: steps fade-up dim
- appears first
- appears second (the first one is dimmed)
:::

::: step zoom 0.8s delay=200ms
![w=400](photo.jpg)
:::
```

- `step` – the whole block is one step (any content: images, tables, code, …).
- `steps` – each list item, or each paragraph/block, is its own step.
- `dim` – dims earlier steps of the container.
- An animation name or duration alone also makes the block a step.

Containers can be nested; the steps appear in the order of the text.

### Positioning and layout

```
::: place bottom right
Placed in the bottom right corner
:::

::: place x=10% y=70% w=35%
At an exact position (in % of the slide)
:::

::: columns 2:1
Left column (two thirds)
|||
Right column
:::

::: center
Centered text
:::
```

- `place` with `top`, `middle`, `bottom` and `left`, `center`, `right` (the missing direction
  is centered); `x=`, `y=`, `w=` for exact positions and width.
- `left`, `center`, `right` without `place` align the text.
- `columns` (equal width) or a ratio like `2:1` or `1:1:1`; `|||` starts the next column.
- `size=1.3` sets the font size (relative), `box` draws a card, `small`, `large`, `muted`.
- All words can be combined, e.g. `::: place bottom left step fade-right`.

### Images

- `![w=400](photo.jpg)`, `![h=300](…)`, `![w=50% center](…)` – size and alignment.
- `![bg](photo.jpg)` – background image filling the slide; `![bg contain](…)` fits it in.
- `![bg left 40%](photo.jpg)` / `![bg right](…)` – image on one side, content on the other.

Relative paths are taken from the folder of the file; in notes from the home folder.
`~/Pictures/x.jpg` works everywhere.

### Presenter notes

Everything after a line `???` on a slide, and HTML comments that are no settings, are
**presenter notes**. They are only visible in the preview and the presenter view.

### Presenting

| Shortcut | Action |
|---|---|
| Ctrl+F5 | Start presentation from the beginning |
| Shift+F5 | Start from the slide at the cursor |
| Ctrl+Shift+F5 | Present in a window (rehearsal) |

The ► button in the header bar offers the same. With **two or more screens** the audience
sees the slides fullscreen on the other screen, while the screen with QuickNotes shows the
**presenter view**: current slide, the next step or slide, large notes, a timer started with
the presentation (remaining time with `duration:`) and the clock. With one screen, `P`
switches between slides and presenter view.

| Key | During the presentation |
|---|---|
| → ↓ Space PgDn Enter, click | Next step or slide |
| ← ↑ PgUp Backspace, right click | Previous step or slide |
| Home / End | First / last slide |
| number + Enter | Go to slide |
| B or . / W or , | Black / white screen |
| P | Presenter view on/off (one screen) |
| S | Swap screens |
| F | Fullscreen on/off |
| T / R | Pause / reset the timer |
| + / − | Notes font size |
| H | Show the keys |
| Esc | End presentation |

*Export as HTML* writes a single file that plays the presentation in any browser (keyboard,
animations, `F` for fullscreen). *Export as PDF* creates one page per slide with all steps
shown, *Export as Image* one image per slide (SVG, PNG or JPG).

## Developer tools {#devtools}

The **developer tools** (button in the header bar, and the *Developer Tools* submenu of the
editor's right-click menu) work on the selection, or on the whole tab when nothing is
selected:

- **Beautify / Minify** JSON, MongoDB / JSON5, XML, YAML and SQL (Ctrl+Alt+B / Ctrl+Alt+M
  detect the format)
- **Convert** JSON ↔ YAML, mongosh ↔ Extended JSON, PHP array ↔ Extended JSON / mongosh
- **Lines:** sort, reverse, remove duplicates or empty lines, trim trailing whitespace
- **Encode / Decode:** Base64, URL, JSON strings
- **Case:** UPPERCASE, lowercase

## Emmet {#emmet}

In **HTML, XML, XSLT, CSS, SCSS and Less** tabs, [Emmet](https://emmet.io) abbreviations
expand into code. Type an abbreviation and press **Tab** (or Ctrl+E):

| Abbreviation | Result |
|---|---|
| `ul>li.item$*3` | a list with three items `item1` … `item3` |
| `div#main.box>p{Hello}` | nested elements with id, class and text |
| `a[href]`, `input:email`, `!` | attributes, typed inputs, an HTML5 document |
| `m10`, `p10-20`, `df`, `bd+`, `pos:a` | `margin: 10px;`, padding, `display: flex;`, border, position |

- `>` child, `+` sibling, `^` up, `*` repeat, `$` number, `()` group, `{}` text,
  `\$` a literal dollar sign. Lorem ipsum: `lorem`, `p*3>lorem10`.
- After an expansion, **Tab** and **Shift+Tab** move between the fields (attribute values,
  element contents); typing replaces the selected placeholder.
- Tab only expands real abbreviations: in HTML plain words must be element or snippet names,
  in CSS the cursor has to be inside a rule block. Inside `<style>` CSS abbreviations apply,
  inside `<script>` none.
- **Wrap with abbreviation** (Ctrl+Shift+G) wraps the selection or the current line, e.g.
  with `ul>li*` (one `li` per line).
- Expanding with Tab can be switched off in *Preferences → General*; Ctrl+E always works.
  In other languages (notes, Markdown, code, …) Emmet is not active.

### MongoDB queries and JSON5 {#mongodb}

Notes with MongoDB shell code (`db.orders.aggregate([...])`) or JSON5 (unquoted keys, single
quotes, comments, trailing commas) are detected automatically and get the *MongoDB / JSON5*
highlighting; files ending in `.mongodb`, `.mongodb.js` or `.json5` too.

<!-- if devtools -->
- **Beautify** puts each pipeline stage on its own line and keeps short documents on one line.
  Keys, quotes and comments stay as written. **Minify** removes comments.
- **mongosh → Extended JSON** turns `ObjectId("…")`, `ISODate("…")`, `NumberLong(…)`,
  `NumberDecimal("…")`, `UUID("…")`, `BinData(…)`, `Timestamp(…)`, `MinKey`/`MaxKey` and
  `/regex/` into Extended JSON v2 (relaxed, or *canonical* with explicit number types). For a
  statement like `db.orders.find({...}).limit(5)` the document argument is converted; otherwise
  select the document or pipeline first. Comments are dropped.
- **Extended JSON → mongosh** does the reverse, also for `mongoexport` output (one document per
  line) and for Extended JSON inside shell statements.

**PHP arrays** (MongoDB PHP library, Doctrine ODM) convert to Extended JSON or mongosh syntax and
back: select the array, `$pipeline = [...];`, `return [...];` or a call like
`->aggregate([...])`. Both `[...]` and `array(...)` work.

- BSON classes are converted, with or without namespace: `new ObjectId(…)`, `new UTCDateTime(…)`
  (milliseconds or `new \DateTime('2024-01-01')`), `new \DateTime('…')` /
  `new \DateTimeImmutable('…')` (dates without a time zone count as UTC), `Regex`, `Decimal128`,
  `Int64`, `Timestamp`, `Binary`, `MinKey`, `MaxKey`, `Javascript`.
- Everything else — variables, method calls, constants — becomes a placeholder like
  `"{{$orders}}"` or `{"$oid": "{{$customer->getId()}}"}`. Replace it with a real value before
  running the query; converting back to PHP restores the original code.
- *Extended JSON / mongosh → PHP array* writes short array syntax with single quotes and
  trailing commas and uses the short class names (add the `use MongoDB\BSON\…;` imports).
  Comments are dropped in all conversions.
<!-- end -->

## Geocaching tools {#geocaching}

Select some text, right-click it and open the **Geocaching** submenu. The result is
inserted on a new line **below the selection** and selected, so it can be copied, edited
or run through the next tool right away. Undo (Ctrl+Z) removes it again.

Tools with a `…` open a small dialog with their options and a live preview of the result.
*Copy* puts the result on the clipboard instead of inserting it; parts of the result can also be
selected with the mouse. The options are remembered for the next time.

| Tool | What it does |
|---|---|
| ROT13, ROT5, ROT18, ROT47 | Quick rotations: letters, digits, letters + digits, all printable ASCII characters |
| Caesar Rotation | Rotates letters by any shift (negative = back), or lists all 25 rotations |
| Letter Value | A=1 … Z=26 (or reversed): values, word sums, sum, checksums and the single-digit checksum; also numbers → letters |
| Letter Frequency | How often each letter occurs; optionally digits and the missing letters |
| Scrabble Value | Letter values of German or English Scrabble with sum |
| Prime Alphabet | A=2, B=3, C=5 … Z=101 and back |
| Hexadecimal | Hex ↔ text (`47 45 4F` or `47454F` = GEO), hex ↔ decimal |
| ASCII Codes | Character codes ↔ text (`71 69 79` = GEO, also without separators) |
| Binary | Binary ↔ decimal, binary ↔ text (8 bits per character) |
| Roman Numerals | Both directions, detected automatically; also IIII or IIX |
| Morse Code | Both directions, detected automatically; `-` or `_` as dash |
| Phone Keypad | Multi-tap (`4 33 666`), key + position (`41 32 63`) or key only (`436`) |

**Notes**

- Umlauts: *Letter Value* can count Ä, Ö, Ü, ß as AE, OE, UE, SS, as 27–30 or ignore them.
  German Scrabble has its own tiles for Ä, Ö and Ü.
- Words are separated by `|` in number results (`7 5 15 | 3 1 3 8 5`); `|` or `/` also
  separate words when numbers are turned back into letters.
- The checksum of 1432 is 10, the single-digit checksum is 1.
- Roman numerals from 4000 on: a thousand-fold value is written with an overline (`X̄L̄` =
  40000) or in brackets (`(XL)` = 40000). Both forms are read.

## Security {#security}

### App password

*Preferences → Security → App Password → Set…* protects QuickNotes with a password:

- At every start QuickNotes shows a password prompt first; the tabs open only after the right
  password. *Main menu → Lock QuickNotes* (or *Lock Now* in the preferences) brings the prompt
  back at any time, e.g. before leaving the computer.
- The tabs (titles, file paths, settings of each tab) and all notes are stored **encrypted**
  in `~/.local/share/quicknotes`. Without the password they cannot be read – also not from a
  copy of the folder. General settings like the theme stay readable so the prompt can be shown.
- Files you opened from disk stay unchanged on disk (only unsaved changes to them are encrypted).
- *Change…* sets a new password, *Remove…* stores everything unencrypted again; both ask for the
  current password first.
- How it works: a random 256-bit key encrypts the data (XChaCha20-Poly1305, which also detects
  any change to the files). This key is itself encrypted with a key derived from your password
  with Argon2id (64 MiB, 3 passes), which makes guessing passwords slow. The password itself is
  never stored.
- **A forgotten password cannot be recovered** – not even by the developer. Keep it safe.
  Note: plain-text copies written before the password was set may remain on the disk until they
  are overwritten.

### Tab password

Single notes can get their own password – with or without the app password:

- *Protect with Password…* (main menu or right click on the tab) encrypts the note with a
  password of its own. The tab shows an **open lock** while you can read it.
- *Lock Tab*, and every start of QuickNotes, hides the content: the tab shows a **closed lock**,
  the title *Locked note* (unless you renamed the tab, F2) and a password prompt instead of the
  text. Enter the password to open it again.
- *Change Tab Password…* and *Remove Tab Password…* ask for the current password first.
- Protected notes stay encrypted in backups, closed tabs and on disk (`.locked` files); only
  their password opens them. Files opened from disk cannot be protected – only notes.
- As with the app password, a forgotten tab password cannot be recovered.

## Backup {#backup}

*Back Up Notes…* in *Preferences → Import/Export* writes all tabs, notes and settings into one
file. *Restore Backup…* either **adds** the tabs of a backup to the current ones or
**replaces** everything (the current state is saved to an automatic backup first).

- **Encrypted backups:** when backing up, enter a password (at least 8 characters, twice) to
  encrypt the backup (`.qnbackup`), or choose *Without Password* for a plain `.tar.gz`. Restoring
  an encrypted backup asks for its password. The key is derived with Argon2id, the data is
  encrypted and authenticated with XChaCha20-Poly1305 – without the password the backup cannot
  be read, and a forgotten password cannot be recovered.

## Keyboard shortcuts {#shortcuts}

| Shortcut | Action |
|---|---|
| Ctrl+N / Ctrl+T | New note |
| Ctrl+Shift+N | New calculation |
<!-- if kanban -->
| Ctrl+Shift+K | New Kanban board |
<!-- end -->
| Ctrl+O, Ctrl+S, Ctrl+Shift+S | Open, save, save as |
| Ctrl+W, Ctrl+Shift+T | Close tab, reopen closed tab |
| F2 | Rename tab |
| F5 | Reload file from disk |
| Ctrl+F, Ctrl+H, F3, Shift+F3 | Find, replace, next, previous |
<!-- if devtools -->
| Ctrl+Alt+B, Ctrl+Alt+M | Beautify, minify |
<!-- end -->
| Ctrl+Shift+V | Toggle preview |
| Ctrl+Shift+E, Ctrl+Shift+P | Export as HTML, PDF |
<!-- if slides -->
| Ctrl+F5, Shift+F5 | Start presentation from the beginning, from the current slide |
| Ctrl+Shift+F5 | Present in a window |
<!-- end -->
| Ctrl+B, Ctrl+I, Ctrl+K | Markdown: bold, italic, link |
| Ctrl+Enter | Check/uncheck checklist item, turn lines into a checklist |
| Tab, Shift+Tab, Enter | Tables: next/previous cell, next row |
| Shift+Alt+arrow keys | Tables: move row or column |
| Alt+Z | Toggle word wrap |
| Alt+Enter, Esc | Distraction-free mode on/off, leave |
| Tab / Ctrl+E, Ctrl+Shift+G | Expand Emmet abbreviation, wrap with abbreviation (HTML, CSS, …) |
| Ctrl+, | Preferences |
| Ctrl+mouse wheel, Ctrl +/−/0 | Font size |
| Ctrl+Tab, Ctrl+PgUp/PgDn, Alt+1…9 | Switch tabs |
| F1 | This help |
<!-- if folders -->
| F9 | Notes sidebar with folders |
<!-- end -->
<!-- if slides -->
| Shift+F1 | Syntax help at the cursor (presentations) |
<!-- end -->
| Ctrl+Q | Quit |

[Back to top](#top)
