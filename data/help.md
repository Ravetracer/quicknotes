# QuickNotes Help {#top}

**Contents:** [Notes & tabs](#notes) · [Calculations](#calc) · [Templates](#templates) ·
[Previews & export](#previews) · [Tools](#tools) · [Backup](#backup) · [Shortcuts](#shortcuts)

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

## Tools {#tools}

The *Format & tools* menu (header bar) works on the selection, or on the whole tab when
nothing is selected:

- **Beautify / Minify** JSON, XML, YAML and SQL (Ctrl+Alt+B / Ctrl+Alt+M detect the format)
- **Convert** JSON ↔ YAML
- **Lines:** sort, reverse, remove duplicates or empty lines, trim trailing whitespace
- **Encode / Decode:** Base64, URL, JSON strings
- **Case:** UPPERCASE, lowercase

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
| Alt+Z | Toggle word wrap |
| Ctrl+mouse wheel, Ctrl +/−/0 | Font size |
| Ctrl+Tab, Ctrl+PgUp/PgDn, Alt+1…9 | Switch tabs |
| F1 | This help |
| Ctrl+Q | Quit |

[Back to top](#top)
