# QuickNotes Help {#top}

**Contents:** [Notes & tabs](#notes) · [Calculations](#calc) · [Templates](#templates) ·
[Previews & export](#previews) · [Presentations](#slides) · [Tools](#tools) · [Geocaching](#geocaching) · [Backup](#backup) ·
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
- The editor font and its size are set in the status bar (or with Ctrl+mouse wheel).
- The interface language (English, Deutsch or the system language) is chosen in the main menu
  under *Language* and applies after a restart.

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

Markdown, reStructuredText, PlantUML and HTML files can be shown with a live preview next to
the text (Ctrl+Shift+V). Markdown and reStructuredText can be exported as **HTML**
(Ctrl+Shift+E) or **PDF** (Ctrl+Shift+P) from the main menu. Relative images are taken from
the folder of the document.

PlantUML needs a local PlantUML installation (`plantuml` command or a `plantuml*.jar` in
`~/bin`).

## Presentations {#slides}

A **presentation** is Markdown split into slides. Create one with *Main menu → New
Presentation* – it starts with a short example. Files named `*.slides.md`, and Markdown files
with `presentation: true` in their front matter, open as presentations; any Markdown tab can
be switched to *Presentation* in the status bar.

The preview shows the slide at the cursor together with its notes; **‹ ›** jump to the
previous/next slide in the text, **►** plays the slide's animations.

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
shown.

## Tools {#tools}

The *Format & tools* menu (header bar) works on the selection, or on the whole tab when
nothing is selected:

- **Beautify / Minify** JSON, XML, YAML and SQL (Ctrl+Alt+B / Ctrl+Alt+M detect the format)
- **Convert** JSON ↔ YAML
- **Lines:** sort, reverse, remove duplicates or empty lines, trim trailing whitespace
- **Encode / Decode:** Base64, URL, JSON strings
- **Case:** UPPERCASE, lowercase

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

## Backup {#backup}

*Back Up Notes…* in the main menu writes all tabs, notes and settings into one `.tar.gz`
file. *Restore Backup…* either **adds** the tabs of a backup to the current ones or
**replaces** everything (the current state is saved to an automatic backup first).

## Keyboard shortcuts {#shortcuts}

| Shortcut | Action |
|---|---|
| Ctrl+N / Ctrl+T | New note |
| Ctrl+Shift+N | New calculation |
| Ctrl+O, Ctrl+S, Ctrl+Shift+S | Open, save, save as |
| Ctrl+W, Ctrl+Shift+T | Close tab, reopen closed tab |
| F2 | Rename tab |
| F5 | Reload file from disk |
| Ctrl+F, Ctrl+H, F3, Shift+F3 | Find, replace, next, previous |
| Ctrl+Alt+B, Ctrl+Alt+M | Beautify, minify |
| Ctrl+Shift+V | Toggle preview |
| Ctrl+Shift+E, Ctrl+Shift+P | Export as HTML, PDF |
| Ctrl+F5, Shift+F5 | Start presentation from the beginning, from the current slide |
| Ctrl+Shift+F5 | Present in a window |
| Alt+Z | Toggle word wrap |
| Ctrl+mouse wheel, Ctrl +/−/0 | Font size |
| Ctrl+Tab, Ctrl+PgUp/PgDn, Alt+1…9 | Switch tabs |
| F1 | This help |
| Ctrl+Q | Quit |

[Back to top](#top)
