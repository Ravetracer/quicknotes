# QuickNotes-Hilfe {#top}

**Inhalt:** [Notizen & Tabs](#notes) · [Checklisten](#checklists) · [Tabellen](#tables) · [Formatierung](#formatting) · [Formeln & Noten](#notation) · [Diagramme](#charts) · [Kanban](#kanban) · [Berechnungen](#calc) · [Vorlagen](#templates) ·
[Vorschau & Export](#previews) · [Präsentationen](#slides) · [Entwicklerwerkzeuge](#devtools) · [Emmet](#emmet) · [Geocaching](#geocaching) ·
[Sicherheit](#security) · [Sicherung](#backup) ·
[Tastenkürzel](#shortcuts)

## Notizen & Tabs {#notes}

Jeder Tab ist entweder eine **Notiz** oder eine **geöffnete Datei**.

- **Notizen** werden automatisch gespeichert – ein Speichern als Datei ist nicht nötig. Nach
  dem Beenden und erneuten Starten von QuickNotes sind alle Tabs wieder da, samt Cursorposition.
- **Geöffnete Dateien** verhalten sich wie in jedem Editor: *Speichern* (Strg+S) schreibt sie
  zurück. Ungespeicherte Änderungen behält QuickNotes, bis sie gespeichert oder verworfen
  werden – auch über Neustarts hinweg.
- Eine geschlossene Notiz lässt sich mit der Schaltfläche **Rückgängig** oder mit
  *Geschlossenen Tab wiederherstellen* (Strg+Umschalt+T) zurückholen.
- Der Tab-Titel ist die erste Zeile einer Notiz. *Tab umbenennen* (F2) legt einen festen Titel fest.
- Die Syntaxhervorhebung wird automatisch gewählt und kann in der Statusleiste geändert werden.
- Die Statusleiste zählt Wörter, Zeichen (ohne Zeilenumbrüche) und Absätze des Tabs bzw. der
  Auswahl, wenn Text markiert ist.
- Tabs sind je nach Art eingefärbt: Code, Dokumente (Markdown, reStructuredText),
  Präsentationen, Berechnungen und Bilder haben jeweils eine eigene Farbe; einfache Notizen
  behalten das normale Aussehen. Die Farben lassen sich in den Einstellungen anpassen.
- Tabs werden oben oder in einer Seitenleiste links angezeigt (*Einstellungen → Tabs*). Die
  Breite der Seitenleiste lässt sich ziehen; Tabs werden per Drag-and-drop umsortiert und mit
  der mittleren Maustaste geschlossen.
- Der **ablenkungsfreie Modus** (Alt+Enter, Esc zum Verlassen) zeigt nur den Text – im
  Vollbild, zentriert, ohne Leisten, Tabs, Vorschau und Zeilennummern.

### Einstellungen

*Hauptmenü → Einstellungen* (Strg+,) legt Editor-Schrift, Schriftgröße und Ligaturen, die
Sprache der Oberfläche (gilt nach einem Neustart), das Farbdesign, die Tab-Position und die
Tab-Farben fest und bietet Sicherung und Wiederherstellung der Notizen (*Import/Export*).

*Funktionen* schaltet optionale Teile von QuickNotes ein und aus: Diagramme, Präsentationen,
Kanban-Boards, die Geocaching-Werkzeuge, PlantUML und die Entwicklerwerkzeuge. Ausgeschaltete Funktionen verschwinden
aus den Menüs, der Werkzeugleiste und dieser Hilfe; Dokumente bleiben, wie sie sind
(Präsentationen und Boards öffnen sich dann als Markdown, Diagramme erscheinen als Code). Neue
Installationen starten mit allen ausgeschaltet.

*Designs* enthält das dunkle und helle Standarddesign sowie 165 Designs aus Dracula, den
Dayle-Rees-Farbschemata und dem VSCode Ultimate Themes Pack. Ein Klick wendet ein Design sofort
auf Editor, Fenster und Vorschau an; Strg+F sucht in den Einstellungen nach Designnamen. Die Schriftgröße lässt sich auch
mit Strg+Mausrad oder Strg +/−/0 ändern.

## Checklisten {#checklists}

Notizen, Markdown-Dokumente und Präsentationen verstehen Aufgabenlisten im üblichen
Markdown-Format, sodass sie überall lesbar bleiben (GitHub, Obsidian, …):

```
- [ ] offener Eintrag
- [x] erledigter Eintrag
```

- **Klick auf das Kästchen** `[ ]` im Editor oder auf die Checkbox in der Markdown-Vorschau
  hakt einen Eintrag ab oder wieder an. Erledigte Einträge werden durchgestrichen.
- **Strg+Enter** hakt den Eintrag in der Cursorzeile ab (bzw. alle markierten; sind schon alle
  erledigt, werden sie wieder geöffnet). Zeilen, die noch keine Einträge sind, werden zu
  Einträgen – ein paar Zeilen Ideen markieren und Strg+Enter drücken ergibt eine Checkliste.
- **Enter** am Ende eines Eintrags beginnt den nächsten, offenen Eintrag; Enter in einem leeren
  Eintrag beendet die Liste. Das funktioniert auch für normale Aufzählungen und nummerierte Listen.
- Die Statusleiste zeigt den Fortschritt, z. B. *✓ 3 von 7 erledigt* (der Auswahl, falls
  vorhanden). Einträge in Codeblöcken zählen nicht mit.

## Tabellen {#tables}

Markdown-Tabellen (in Notizen, Markdown und Präsentationen) werden wie mit *Advanced Tables*
aus Obsidian bearbeitet: Die Tabelle wird beim Bewegen durch die Zellen automatisch
formatiert, sodass die Spalten immer bündig sind – auch mit Umlauten und breiten Zeichen.

- Eine Tabelle beginnt man mit `| Name` und **Tab**: Die Kopfzeile bekommt eine neue Spalte und
  die Trennzeile wird ergänzt. Oder *Tabelle einfügen* im Kontextmenü.
- **Tab** / **Umschalt+Tab** springen zur nächsten / vorigen Zelle und markieren ihren Text,
  sodass Tippen ihn ersetzt. Tab in der letzten Zelle einer Zeile fügt eine neue Spalte an.
- **Enter** springt in derselben Spalte zur nächsten Zeile und fügt am Ende eine Zeile an.
  Enter in einer leeren letzten Zeile entfernt sie und verlässt die Tabelle.
- In einer Tabelle zeigt das Kontextmenü (Rechtsklick in eine Zelle oder Umschalt+F10) nur die
  Tabellenbefehle: formatieren,
  Zeilen und Spalten einfügen, verschieben und löschen, die Zeilen nach der Spalte
  **sortieren** (Zahlen numerisch, Text alphabetisch) und die Spalte links, zentriert oder
  rechts **ausrichten** (`:--`, `:-:`, `--:`).
- **Umschalt+Alt+↑/↓** verschieben die Zeile, **Umschalt+Alt+←/→** die Spalte.
- *Formatieren* (Strg+Alt+B) formatiert in einem Markdown-Dokument alle Tabellen auf einmal.
- **Daten importieren** (Tabellenmenü der Werkzeugleiste oder Kontextmenü): Eine CSV/TSV-Datei
  oder Tabellenkalkulation (Excel `.xlsx`/`.xls`, LibreOffice `.ods`) wird zur
  Markdown-Tabelle. Wählbar sind das Tabellenblatt, das Trennzeichen (wird erkannt, auch
  Excels `;`) und ob die erste Zeile die Kopfzeile ist. Datumswerte werden zu `JJJJ-MM-TT`.
- **Als Tabelle einfügen** macht aus kopierten Zellen einer Tabellenkalkulation (oder anderem
  Text mit Tabs/Kommas) eine Tabelle.

## Formatierung {#formatting}

Markdown-Dokumente und Präsentationen haben über dem Text eine **Formatierungsleiste**. Sie
schreibt das Markdown für dich, die Syntax musst du dir nicht merken:

- **Überschriften** 1–6 (und zurück zu normalem Text), **fett** (Strg+B), *kursiv* (Strg+I),
  ~~durchgestrichen~~ und `Code im Text`. Ohne Auswahl wirken sie auf das Wort am Cursor.
  Nochmal anwenden entfernt die Formatierung wieder.
- **Textfarbe**: setzt die Auswahl in `<span style="color: …">`, sichtbar in der Vorschau und
  im HTML-/PDF-Export. *Farbe entfernen* nimmt sie wieder weg.
- Aufzählungen, nummerierte Listen, Checklisten und Zitate für die markierten Zeilen (nochmal
  zum Entfernen), Codeblöcke, Trennlinien und Tabellen.
- **Link** (Strg+K) und **Bild**: Die Auswahl wird zum Text, eine markierte URL zum Ziel.
<!-- if charts -->
- **Diagramm** (ein Diagramm aus der Tabelle darüber, mit Beispieldaten, wenn es keine gibt),
  siehe [Diagramme](#charts).
<!-- end -->
- **∑ Formel** (im Text oder als Block) und **♫ Notenschrift** (eine ABC-Melodie), siehe unten.

*Hauptmenü → Formatierungsleiste* blendet die Leiste aus; die Tastenkürzel funktionieren weiter.

## Formeln & Noten {#notation}

Markdown-Dokumente und Präsentationen können **LaTeX-Formeln** und **Notenschrift** enthalten.
Beides erscheint in der Vorschau, in Präsentationen und im HTML-/PDF-Export (ohne Internet).

**Formeln** verwenden die LaTeX-Mathe-Syntax (gesetzt mit KaTeX):

- `$E = mc^2$` – eine Formel im Text. Ein `$` mit Leerzeichen dahinter öffnet keine Formel,
  eins mit Leerzeichen davor schließt keine – Preise wie $5 oder $10 bleiben also, wie sie sind.
- `$$ \int_0^1 x^2\,dx $$` – eine eigenständige, zentrierte Formel.
- Ein Codeblock mit der Sprache `math` – praktisch für längere Formeln über mehrere Zeilen:

      ```math
      \begin{pmatrix} a & b \\ c & d \end{pmatrix}
      ```

Brüche `\frac{a}{b}`, Wurzeln `\sqrt{x}`, Summen `\sum_{k=1}^n`, Integrale, Matrizen,
griechische Buchstaben `\alpha`, … werden unterstützt; ein Fehler zeigt die Formel rot an, statt
die Seite zu stören.

**Noten** werden in [ABC-Notation](https://abcnotation.com) in einem Codeblock mit der Sprache
`abc` geschrieben (gesetzt mit abcjs):

      ```abc
      X:1
      T:Tonleiter
      M:4/4
      L:1/4
      K:D
      DEFG|ABcd|
      ```

Die Kopfzeilen legen Nummer (`X`), Titel (`T`), Takt (`M`), Standard-Notenlänge (`L`) und
Tonart (`K`, immer zuletzt) fest; danach folgt die Melodie: `C D E` sind Noten, `c d e` eine
Oktave höher, `C2` doppelt so lang, `|` ein Taktstrich, `^` / `_` Kreuz / B, `z` eine Pause.

## Diagramme {#charts}

Markdown-Dokumente und Präsentationen können **Diagramme aus ihren Tabellen** zeichnen (mit
Apache ECharts; in der Vorschau, in Präsentationen und im HTML/PDF-Export, ohne Internet). Ein
Codeblock mit der Sprache `chart` enthält die Einstellungen als `Schlüssel: Wert`-Zeilen; die
Daten kommen aus einer Tabelle des Dokuments – standardmäßig der nächsten **über** dem Block:

    | Monat | Umsatz  | Kosten |
    |-------|---------|--------|
    | Jan   | 1.200 € | 800 €  |
    | Feb   | 1.500 € | 950 €  |

      ```chart
      type: bar
      title: Umsatz und Kosten
      ```

Der **Diagramm-Knopf** der Werkzeugleiste fügt so einen Block ein (mit einer Beispieltabelle,
wenn darüber keine Tabelle steht). Fehler erscheinen rot an der Stelle des Diagramms.

**Welche Tabelle:** `table: above` (Standard; gibt es darüber keine, wird die darunter
genommen), `table: below`, `table: 2` (die zweite Tabelle des Dokuments) oder `table: Umsatz`
(die erste Tabelle unter der Überschrift „Umsatz“).

**Zahlen in Zellen** dürfen geschrieben sein, wie man will: `1.234,56` und `1,234.56`, `12 %`,
`€ 5`, `5 kg`, `(120)` für −120. Leere Zellen, `-` und `n/a` sind Lücken. Ist eine Zahl
mehrdeutig (`1.234`), entscheiden die anderen Zahlen der Spalte, dann `decimal: ","` oder
`"."`, dann die Sprache der App. Eine Einheit, die alle Zahlen teilen, erscheint an der Achse.
Datumswerte: `2026-03-01`, `01.03.2026` oder `2026-03`.

**Diagrammtypen** und die Spalten, die sie nutzen (Spalten per Überschrift oder Nummer; ohne
Angaben liefert die erste Spalte die Namen/Kategorien und alle Zahlenspalten die Werte):

| Typ | Daten |
|-----|-------|
| `line`, `area`, `bar` | Spalte `x` (Kategorien; Datumswerte ergeben eine Zeitachse) und Spalten `y`; `group:` bildet Reihen aus den Werten einer Spalte (lange Tabellen) |
| `pie`, `donut`, `rose` | Namen und eine Wertespalte (mehrere Wertespalten: mehrere Kreise) |
| `scatter`, `bubble` | Zahlen `x` und `y`; `size` (Blasen), `label`, `group` |
| `radar` | die Zeilen sind die Achsen, jede Wertespalte eine Fläche |
| `funnel` | Namen und Werte |
| `gauge` | ein Tachometer je Zeile – oder ein fester Wert: `value: 72` (`min`, `max`, `unit`) |
| `boxplot` | ein Boxplot je Zahlenspalte, oder `x: Gruppe` mit einer Wertespalte |
| `candlestick` | Datum, Eröffnung, Hoch, Tief, Schluss (per Überschrift oder in dieser Reihenfolge) |
| `heatmap` | eine Matrix (Zeilennamen, eine Spalte je x-Wert) – oder Spalten `x`, `y` und `value` |
| `calendar` | eine Datums- und eine Wertespalte; ein Kalender je Jahr |
| `sankey`, `graph` | Spalten `source`, `target` und `value` (`directed: true`, `layout: circular` für Graphen) |
| `treemap`, `sunburst`, `tree` | Pfadspalten (Region / Land / Stadt) und ein Wert – oder Spalten `name` und `parent` |
| `parallel` | eine Achse je Spalte (`y: [...]` zur Auswahl), `group` für Farben |
| `themeriver` | Datumswerte/Zahlen und Wertespalten (oder `group` und `value`) |

**Einstellungen:** `title`, `subtitle`, `stack: true`, `horizontal: true`, `smooth: true`,
`step: true`, `labels: true` (Werte anzeigen), `legend: top|bottom|left|right|false`,
`zoom: true`, `sort: asc|desc`, `y2: Spalte` (zweite Achse rechts, als Linie), `min`, `max`,
`log: true`, `unit: "€"`, `height: 400`, `theme: light|dark` (Standard: wie die Seite),
`palette:` ein Name (`default`, `gnome`, `macarons`, `vintage`, `roma`, `shine`,
`infographic`, `royal`, `sakura`, `caravan`, `azul`, `helianthus`, `blue`, `green`, `red`,
`gray`) oder eine Liste von Farben, `animation: true`. Alles Weitere, was ECharts kann, kommt
unter `echarts:` und wird ins Diagramm übernommen, z. B.
`echarts: { yAxis: [{ splitLine: { show: false } }] }`.

**Vorgaben für alle Diagramme** eines Dokuments stehen im Front Matter:

    ---
    charts:
      palette: vintage
      height: 300
      decimal: ","
    ---

## Kanban-Boards {#kanban}

*Hauptmenü → Neues Kanban-Board* (Strg+Umschalt+K) legt ein Board mit Spalten an (z. B.
*Zu erledigen*, *In Arbeit*, *Erledigt*) und optionalen **Swimlanes** (Zeilen quer über alle
Spalten, z. B. je Projekt oder Person).

- **Karten:** *+ Karte hinzufügen* in einer Zelle; ein Klick auf eine Karte bearbeitet sie – die
  erste Zeile ist der Titel, weitere Zeilen (Umschalt+Enter) die Beschreibung; Enter speichert,
  Esc bricht ab. Die Checkbox markiert eine Karte als erledigt, × löscht sie. Karten lassen sich
  mit der Maus an eine andere Stelle, in eine andere Spalte oder Swimlane ziehen.
- **Spalten:** *+ Spalte* fügt eine hinzu. Den Spaltenkopf ziehen, um die Reihenfolge zu ändern;
  das ⋯-Menü benennt sie um (oder Doppelklick auf den Kopf), setzt **Farbe** und **WIP-Limit**
  (der Zähler wird rot, wenn die Spalte mehr Karten enthält), verschiebt oder löscht sie.
- **Swimlanes:** *+ Swimlane* fügt eine hinzu (die erste bekommt dann ebenfalls einen Namen).
  Den Kopf ziehen zum Umsortieren, ▾ klappt sie ein, das ⋯-Menü benennt um, verschiebt, löscht.
- Klick auf den Board-Titel benennt das Board um. Strg+Z / Strg+Umschalt+Z machen Änderungen am
  Board rückgängig bzw. wiederholen sie.
- Die Statusleiste zählt die Karten und die erledigten.

Ein Board wird als lesbares Markdown gespeichert (Swimlanes als `#`-, Spalten als
`##`-Überschriften, Karten als `- [ ]`-Einträge, Farben und Limits im Front Matter) und bleibt so
in jedem Editor und auf dem Handy nutzbar. *Vorschau* (Strg+Umschalt+V) zeigt diese Quelle neben
dem Board; Änderungen daran aktualisieren das Board. Als Datei gespeichert bekommen Boards die
Endung `.kanban.md`. Die Board-Ansicht schreibt die Quelle in ihrem eigenen Format – freier Text
zwischen den Karten wird Teil der Karte darüber.

## Berechnungen {#calc}

Eine **Berechnung** (Rechenblatt) wertet jede Zeile aus und zeigt das Ergebnis am rechten Rand
der Zeile. Neu anlegen mit *Neue Berechnung* (Strg+Umschalt+N) oder für einen beliebigen Tab in
der Statusleiste **Calculation** als Syntax wählen. Die Ergebnisse werden beim Tippen
aktualisiert; **ein Klick auf ein Ergebnis kopiert es**.

```
a = 5            # Variablen definieren
b = 10
d = a + b        # = 15
d * 2            # = 30
(a + b) / 4      # = 3.75
```

**Regeln**

- `name = ausdruck` definiert eine Variable. Namen beginnen mit einem Buchstaben und dürfen
  Buchstaben, Ziffern und `_` enthalten. Groß- und Kleinschreibung ergeben verschiedene
  Variablen (`a` ≠ `A`).
- Die Zeilen werden von oben nach unten ausgewertet; eine Variable kann später neu definiert
  werden.
- Eine Zeile, die nur aus einem Ausdruck besteht, zeigt dessen Wert. Reine Zuweisungen wie
  `a = 5` zeigen nichts an, weil der Wert schon dasteht.
- `#` oder `//` leiten einen Kommentar ein. Zeilen, die keine Berechnung sind (normaler Text),
  werden ignoriert – so lassen sich Notizen zwischen die Berechnungen schreiben.
- Fehler erscheinen in Rot, z. B. *unbekannte Variable „x“* oder *Division durch null*.

**Operatoren**

| Operator | Bedeutung | Beispiel |
|---|---|---|
| `+` `-` | Addition, Subtraktion | `a + b - 1` |
| `*` `×` `·` | Multiplikation | `a * 3` |
| `/` `÷` `:` | Division | `a / 4` |
| `%` | Rest (Modulo) | `17 % 5` = 2 |
| `^` `**` | Potenz | `2 ^ 10` = 1024 |
| `!` | Fakultät | `5!` = 120 |
| `( )` `[ ]` | Klammern | `(a + b) * [c - 1]` |

Eine Zahl oder schließende Klammer direkt vor einem Namen oder einer Klammer multipliziert:
`2a` = `2 * a`, `3(a + b)` = `3 * (a + b)`. Dezimaltrennzeichen ist der Punkt (`1.5`).

**Funktionen**

| Funktion | Beschreibung |
|---|---|
| `sqrt(x)` | Quadratwurzel |
| `abs(x)` | Betrag |
| `round(x)`, `round(x, n)` | runden, optional auf *n* Nachkommastellen |
| `floor(x)`, `ceil(x)`, `trunc(x)` / `int(x)` | abrunden, aufrunden, Nachkommastellen abschneiden |
| `min(…)`, `max(…)`, `sum(…)`, `avg(…)` | aus beliebig vielen Werten |
| `qs(x)` / `digitsum(x)` | Quersumme: `qs(1234)` = 10 |
| `iqs(x)` / `digitroot(x)` | iterierte Quersumme bis auf eine Ziffer: `iqs(98765)` = 8 |
| `sin`, `cos`, `tan`, `asin`, `acos`, `atan` | Trigonometrie im **Bogenmaß** |
| `rad(x)`, `deg(x)` | Grad → Bogenmaß, Bogenmaß → Grad: `sin(rad(30))` = 0.5 |
| `ln(x)`, `log(x)`, `log2(x)`, `exp(x)` | Logarithmen (e, 10, 2) und e^x |
| `pi` | 3.14159… (eine Variable namens `pi` hat Vorrang) |

## Vorlagen {#templates}

Eine Zeile, die mit `>` beginnt, ist eine **Vorlage**. Ihr ausgefüllter Text erscheint als
Ergebnis und lässt sich per Klick kopieren – ideal für Koordinaten beim Geocaching, aber
nützlich für alles, was berechnete Werte zu einem Text zusammensetzt.

**Buchstaben als Platzhalter** – jeder einzelne Buchstabe, der eine definierte Variable ist,
wird durch ihren Wert ersetzt; alle anderen Zeichen bleiben unverändert:

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

`N` bleibt hier `N`, weil es keine Variable `N` gibt. Soll ein Buchstabe erhalten bleiben,
obwohl eine solche Variable existiert, wird ihm ein Backslash vorangestellt: `\N`.

**Berechnungen in geschweiften Klammern** – `{…}` enthält einen beliebigen Ausdruck:

```
a = 7
b = 3
> N 52° {a*b}.{a*100+b} E 013° {b*4}.{(a+b)*12}
                     → N 52° 21.703 E 013° 12.120
```

**Formatierung** – nach einem Doppelpunkt innerhalb der Klammern:

| Format | Bedeutung | Beispiel (x = 7.256) |
|---|---|---|
| `{x:3}` | mindestens 3 Stellen vor dem Punkt, mit Nullen aufgefüllt | `007.256` |
| `{x:.2}` | genau 2 Nachkommastellen (gerundet) | `7.26` |
| `{x:2.1}` | beides | `07.3` |

Minuten in Koordinaten brauchen meist zwei Stellen und drei Nachkommastellen, z. B.
`> N 52° {m:2}.{t:3}`. Innerhalb der Klammern `/` zum Dividieren verwenden (der Doppelpunkt
leitet das Format ein).

## Vorschau & Export {#previews}

<!-- if plantuml -->
Markdown-, reStructuredText-, PlantUML- und HTML-Dateien lassen sich mit einer Live-Vorschau
<!-- end -->
<!-- if !plantuml -->
Markdown-, reStructuredText- und HTML-Dateien lassen sich mit einer Live-Vorschau
<!-- end -->
neben dem Text anzeigen (Strg+Umschalt+V). Markdown und reStructuredText können über das
Hauptmenü oder das Kontextmenü des Editors (rechte Maustaste) als **HTML**
(Strg+Umschalt+E) oder **PDF** (Strg+Umschalt+P) exportiert werden.
Relative Bilder werden aus dem Ordner des Dokuments geladen.

<!-- if plantuml -->
PlantUML benötigt eine lokale PlantUML-Installation (Befehl `plantuml` oder eine
`plantuml*.jar` in `~/bin`).
<!-- end -->

## Präsentationen {#slides}

Eine **Präsentation** ist Markdown, das in Folien aufgeteilt ist. Neu anlegen mit
*Hauptmenü → Neue Präsentation* – sie beginnt mit einem kurzen Beispiel. Dateien mit dem
Namen `*.slides.md` und Markdown-Dateien mit `presentation: true` im Vorspann werden als
Präsentation geöffnet; jeder Markdown-Tab lässt sich in der Statusleiste auf *Presentation*
umstellen.

Die Vorschau zeigt die Folie am Cursor mit ihren Notizen; **‹ ›** springen im Text zur
vorherigen/nächsten Folie, **►** spielt die Animationen der Folie ab.

Beim Tippen schlägt QuickNotes vor, was an der Schreibmarke passt – nach `::: `, im Front
Matter, in `<!-- … -->`-Einstellungen und in Bild-Optionen `![…]` –, jeweils mit kurzer
Erklärung (Strg+Leertaste zeigt die Liste jederzeit). **Umschalt+F1** (oder *Syntax-Hilfe* im
Kontextmenü) öffnet eine Übersicht aller Optionen an der Schreibmarke.

### Folien und Einstellungen

Eine Zeile mit nur `---` beginnt eine neue Folie (in Code-Blöcken bleibt sie Code).
Einstellungen für die ganze Präsentation stehen in einem Vorspann ganz am Anfang:

```
---
presentation: true
title: Mein Vortrag
theme: default        # default, dark, ocean, sunset
aspect: 16:9          # 16:9, 4:3, 16:10
transition: fade 500ms
step: fade-up 400ms   # Standard-Animation der Schritte
paginate: true        # Foliennummern
header: Text oben
footer: Text unten
duration: 20m         # Vortragsdauer, wird als Restzeit angezeigt
style: |
  .qn-slide h1 { text-transform: uppercase; }
---
```

Einstellungen einer einzelnen Folie stehen als HTML-Kommentar irgendwo auf der Folie,
mehrere durch `;` getrennt:

```
<!-- transition: zoom 0.8s; background: #1e3a5f; color: white; class: lead -->
```

Folien-Einstellungen: `transition`, `step`, `theme`, `background` (Farbe, Verlauf),
`color`, `class`, `align` (left, center, right), `valign` (top, middle, bottom), `header`,
`footer`, `paginate`. Klassen: `lead` (zentrierte Titelfolie), `invert` (vertauschte Farben).

### Animationen

Dauern werden als `400ms` oder `0.8s` angegeben oder als Tempo `fast`, `normal`, `slow`.

- **Folienübergänge** (`transition:`): `none`, `fade`, `slide-left` (`slide`),
  `slide-right`, `slide-up`, `slide-down`, `cover`, `zoom`, `zoom-out`, `flip`, `cube`,
  `rotate`, `blur`. Beim Zurückblättern läuft der Übergang rückwärts.
- **Schritt-Animationen**: `fade`, `fade-up`, `fade-down`, `fade-left`, `fade-right`, `zoom`,
  `zoom-out`, `flip`, `rotate`, `blur`, `bounce`, `drop`, `wipe`, `none` (einfach erscheinen).

### Schritt für Schritt

Inhalte in einem **Container** – eine Zeile `:::` mit Wörtern, geschlossen durch eine Zeile
`:::` – erscheinen per Klick oder Tastendruck:

```
::: steps fade-up dim
- erscheint zuerst
- erscheint als Zweites (der erste Punkt wird abgeblendet)
:::

::: step zoom 0.8s delay=200ms
![w=400](foto.jpg)
:::
```

- `step` – der ganze Block ist ein Schritt (beliebiger Inhalt: Bilder, Tabellen, Code, …).
- `steps` – jeder Listenpunkt bzw. jeder Absatz/Block ist ein eigener Schritt.
- `dim` – blendet frühere Schritte des Containers ab.
- Ein Animationsname oder eine Dauer allein macht den Block ebenfalls zum Schritt.

Container lassen sich verschachteln; die Schritte erscheinen in der Reihenfolge des Textes.

### Positionieren und Layout

```
::: place bottom right
Unten rechts platziert
:::

::: place x=10% y=70% w=35%
An einer genauen Position (in % der Folie)
:::

::: columns 2:1
Linke Spalte (zwei Drittel)
|||
Rechte Spalte
:::

::: center
Zentrierter Text
:::
```

- `place` mit `top`, `middle`, `bottom` und `left`, `center`, `right` (die fehlende Richtung
  wird zentriert); `x=`, `y=`, `w=` für genaue Position und Breite.
- `left`, `center`, `right` ohne `place` richten den Text aus.
- `columns` (gleich breit) oder ein Verhältnis wie `2:1` oder `1:1:1`; `|||` beginnt die
  nächste Spalte.
- `size=1.3` setzt die Schriftgröße (relativ), `box` zeichnet eine Karte, `small`, `large`,
  `muted`.
- Alle Wörter lassen sich kombinieren, z. B. `::: place bottom left step fade-right`.

### Bilder

- `![w=400](foto.jpg)`, `![h=300](…)`, `![w=50% center](…)` – Größe und Ausrichtung.
- `![bg](foto.jpg)` – Hintergrundbild über die ganze Folie; `![bg contain](…)` passt es ein.
- `![bg left 40%](foto.jpg)` / `![bg right](…)` – Bild auf einer Seite, Inhalt auf der
  anderen.

Relative Pfade beziehen sich auf den Ordner der Datei, bei Notizen auf den Home-Ordner.
`~/Bilder/x.jpg` funktioniert überall.

### Notizen für den Vortragenden

Alles nach einer Zeile `???` auf einer Folie sowie HTML-Kommentare, die keine Einstellungen
sind, sind **Notizen**. Sie erscheinen nur in der Vorschau und in der Referentenansicht.

### Präsentieren

| Tastenkürzel | Aktion |
|---|---|
| Strg+F5 | Präsentation von Anfang an starten |
| Umschalt+F5 | Ab der Folie am Cursor starten |
| Strg+Umschalt+F5 | In einem Fenster präsentieren (Probe) |

Dasselbe bietet die ►-Schaltfläche in der Kopfleiste. Mit **zwei oder mehr Bildschirmen**
sieht das Publikum die Folien im Vollbild auf dem anderen Bildschirm, auf dem Bildschirm mit
QuickNotes erscheint die **Referentenansicht**: aktuelle Folie, nächster Schritt bzw. nächste
Folie, große Notizen, eine Uhr, die mit der Präsentation startet (Restzeit mit `duration:`),
und die Uhrzeit. Mit einem Bildschirm wechselt `P` zwischen Folien und Referentenansicht.

| Taste | Während der Präsentation |
|---|---|
| → ↓ Leertaste Bild↓ Enter, Klick | Nächster Schritt bzw. nächste Folie |
| ← ↑ Bild↑ Rücktaste, Rechtsklick | Vorheriger Schritt bzw. vorherige Folie |
| Pos1 / Ende | Erste / letzte Folie |
| Zahl + Enter | Gehe zu Folie |
| B oder . / W oder , | Schwarzer / weißer Bildschirm |
| P | Referentenansicht ein/aus (ein Bildschirm) |
| S | Bildschirme tauschen |
| F | Vollbild ein/aus |
| T / R | Zeit anhalten / zurücksetzen |
| + / − | Schriftgröße der Notizen |
| H | Tasten anzeigen |
| Esc | Präsentation beenden |

*Als HTML exportieren* schreibt eine einzelne Datei, die die Präsentation in jedem Browser
abspielt (Tastatur, Animationen, `F` für Vollbild). *Als PDF exportieren* erzeugt eine Seite
pro Folie mit allen Schritten.

## Entwicklerwerkzeuge {#devtools}

Die **Entwicklerwerkzeuge** (Knopf in der Kopfleiste und das Untermenü *Entwicklerwerkzeuge*
im Kontextmenü des Editors) arbeiten auf der Auswahl oder, wenn nichts ausgewählt ist, auf
dem ganzen Tab:

- **Formatieren / Minimieren** von JSON, MongoDB / JSON5, XML, YAML und SQL (Strg+Alt+B /
  Strg+Alt+M erkennen das Format)
- **Umwandeln** JSON ↔ YAML, mongosh ↔ Extended JSON, PHP-Array ↔ Extended JSON / mongosh
- **Zeilen:** sortieren, umkehren, Duplikate oder leere Zeilen entfernen, Leerzeichen am
  Zeilenende entfernen
- **Kodieren / Dekodieren:** Base64, URL, JSON-Strings
- **Groß-/Kleinschreibung:** GROSSBUCHSTABEN, kleinbuchstaben

## Emmet {#emmet}

In Tabs mit **HTML, XML, XSLT, CSS, SCSS und Less** werden [Emmet](https://emmet.io)-Abkürzungen
zu Code erweitert. Abkürzung tippen und **Tab** (oder Strg+E) drücken:

| Abkürzung | Ergebnis |
|---|---|
| `ul>li.item$*3` | eine Liste mit drei Einträgen `item1` … `item3` |
| `div#main.box>p{Hallo}` | verschachtelte Elemente mit ID, Klasse und Text |
| `a[href]`, `input:email`, `!` | Attribute, Eingabefelder, ein HTML5-Dokument |
| `m10`, `p10-20`, `df`, `bd+`, `pos:a` | `margin: 10px;`, Padding, `display: flex;`, Rahmen, Position |

- `>` Kind, `+` Geschwister, `^` eine Ebene hoch, `*` Wiederholung, `$` Nummer, `()` Gruppe,
  `{}` Text, `\$` ein echtes Dollarzeichen. Blindtext: `lorem`, `p*3>lorem10`.
- Nach dem Erweitern springen **Tab** und **Umschalt+Tab** zwischen den Feldern
  (Attributwerte, Elementinhalte); Tippen ersetzt den markierten Platzhalter.
- Tab erweitert nur echte Abkürzungen: in HTML müssen einzelne Wörter Element- oder
  Snippet-Namen sein, in CSS muss der Cursor in einem Regelblock stehen. In `<style>` gelten
  CSS-Abkürzungen, in `<script>` keine.
- **Mit Abkürzung umschließen** (Strg+Umschalt+G) umschließt die Auswahl oder die aktuelle
  Zeile, z. B. mit `ul>li*` (ein `li` pro Zeile).
- Das Erweitern mit Tab lässt sich unter *Einstellungen → Allgemein* abschalten; Strg+E
  funktioniert immer. In anderen Sprachen (Notizen, Markdown, Code, …) ist Emmet nicht aktiv.

### MongoDB-Abfragen und JSON5

Notizen mit MongoDB-Shell-Code (`db.orders.aggregate([...])`) oder JSON5 (Schlüssel ohne
Anführungszeichen, einfache Anführungszeichen, Kommentare, Komma am Ende) werden automatisch
erkannt und als *MongoDB / JSON5* hervorgehoben; ebenso Dateien mit der Endung `.mongodb`,
`.mongodb.js` oder `.json5`.

<!-- if devtools -->
- **Formatieren** setzt jede Pipeline-Stufe in eine eigene Zeile und lässt kurze Dokumente in
  einer Zeile. Schlüssel, Anführungszeichen und Kommentare bleiben, wie sie sind.
  **Minimieren** entfernt Kommentare.
- **mongosh → Extended JSON** wandelt `ObjectId("…")`, `ISODate("…")`, `NumberLong(…)`,
  `NumberDecimal("…")`, `UUID("…")`, `BinData(…)`, `Timestamp(…)`, `MinKey`/`MaxKey` und
  `/regex/` in Extended JSON v2 um (relaxed oder *kanonisch* mit expliziten Zahlentypen). Bei
  einer Anweisung wie `db.orders.find({...}).limit(5)` wird das Dokument-Argument umgewandelt,
  sonst vorher das Dokument oder die Pipeline auswählen. Kommentare entfallen.
- **Extended JSON → mongosh** wandelt zurück, auch `mongoexport`-Ausgaben (ein Dokument pro
  Zeile) und Extended JSON innerhalb von Shell-Anweisungen.

**PHP-Arrays** (MongoDB-PHP-Bibliothek, Doctrine ODM) lassen sich in Extended JSON oder
mongosh-Syntax umwandeln und zurück: das Array, `$pipeline = [...];`, `return [...];` oder einen
Aufruf wie `->aggregate([...])` auswählen. `[...]` und `array(...)` funktionieren beide.

- BSON-Klassen werden umgewandelt, mit oder ohne Namespace: `new ObjectId(…)`,
  `new UTCDateTime(…)` (Millisekunden oder `new \DateTime('2024-01-01')`), `new \DateTime('…')` /
  `new \DateTimeImmutable('…')` (Datumsangaben ohne Zeitzone gelten als UTC), `Regex`,
  `Decimal128`, `Int64`, `Timestamp`, `Binary`, `MinKey`, `MaxKey`, `Javascript`.
- Alles andere – Variablen, Methodenaufrufe, Konstanten – wird zu einem Platzhalter wie
  `"{{$orders}}"` oder `{"$oid": "{{$customer->getId()}}"}`. Vor dem Ausführen der Abfrage durch
  einen echten Wert ersetzen; beim Zurückwandeln nach PHP entsteht wieder der ursprüngliche Code.
- *Extended JSON / mongosh → PHP-Array* schreibt kurze Array-Syntax mit einfachen
  Anführungszeichen und Komma am Ende und verwendet die kurzen Klassennamen (die
  `use MongoDB\BSON\…;`-Importe ergänzen). Kommentare entfallen bei allen Umwandlungen.
<!-- end -->

## Geocaching-Werkzeuge {#geocaching}

Text markieren, mit der rechten Maustaste anklicken und das Untermenü **Geocaching** öffnen.
Das Ergebnis wird in einer neuen Zeile **unter der Auswahl** eingefügt und markiert, sodass es
sofort kopiert, bearbeitet oder durch das nächste Werkzeug geschickt werden kann. Rückgängig
(Strg+Z) entfernt es wieder.

Werkzeuge mit `…` öffnen einen kleinen Dialog mit ihren Optionen und einer Live-Vorschau des
Ergebnisses. Der Ergebnistext im Dialog lässt sich mit der Maus markieren; die Schaltfläche
*Kopieren* kopiert das Ergebnis in die Zwischenablage, *Darunter einfügen* fügt es unter der
Auswahl ein. Die Optionen werden für das nächste Mal gespeichert.

| Werkzeug | Funktion |
|---|---|
| ROT13, ROT5 (Ziffern), ROT18 (Buchstaben + Ziffern), ROT47 | Schnelle Verschiebungen: Buchstaben, Ziffern, Buchstaben + Ziffern, alle druckbaren ASCII-Zeichen |
| Caesar-Verschiebung | Verschiebt Buchstaben um einen beliebigen Wert (negativ = zurück) oder listet alle 25 Verschiebungen auf |
| Buchstabenwert (BWW) | A=1 … Z=26 (oder umgekehrt): Werte, Buchstabenwortwerte (BWW), Summe, Quersummen und iterierte (einstellige) Quersumme; auch Zahlen → Buchstaben |
| Buchstabenhäufigkeit | Wie oft jeder Buchstabe vorkommt; optional Ziffern und die fehlenden Buchstaben |
| Scrabble-Wert | Buchstabenwerte des deutschen oder englischen Scrabble mit Summe |
| Primzahl-Alphabet | A=2, B=3, C=5 … Z=101 und zurück |
| Hexadezimal | Hex ↔ Text (`47 45 4F` oder `47454F` = GEO), Hex ↔ Dezimal |
| ASCII-Codes | Zeichencodes ↔ Text (`71 69 79` = GEO, auch ohne Trennzeichen) |
| Binär | Binär ↔ Dezimal, Binär ↔ Text (8 Bit pro Zeichen) |
| Römische Zahlen | Beide Richtungen, automatisch erkannt; auch IIII oder IIX |
| Morsecode | Beide Richtungen, automatisch erkannt; `-` oder `_` als Strich |
| Handycode (Tastatur) | Mehrfachtippen (`4 33 666`), Taste + Position (`41 32 63`) oder nur Taste (T9) (`436`) |

**Hinweise**

- Umlaute: *Buchstabenwert (BWW)* kann Ä, Ö, Ü, ß als AE, OE, UE, SS oder als 27–30 zählen
  oder sie ignorieren. Das deutsche Scrabble hat eigene Steine für Ä, Ö und Ü.
- In Zahlenergebnissen werden Wörter durch `|` getrennt (`7 5 15 | 3 1 3 8 5`); beim
  Zurückwandeln von Zahlen in Buchstaben trennen `|` oder `/` ebenfalls Wörter.
- Die Quersumme von 1432 ist 10, die iterierte (einstellige) Quersumme ist 1.
- Römische Zahlen ab 4000: Ein tausendfacher Wert wird mit Überstrich (`X̄L̄` = 40000) oder in
  Klammern (`(XL)` = 40000) geschrieben. Beide Schreibweisen werden gelesen.

## Sicherheit {#security}

### App-Passwort

*Einstellungen → Sicherheit → App-Passwort → Festlegen …* schützt QuickNotes mit einem Passwort:

- Bei jedem Start zeigt QuickNotes zuerst eine Passwortabfrage; die Tabs öffnen sich erst nach
  dem richtigen Passwort. *Hauptmenü → QuickNotes sperren* (oder *Jetzt sperren* in den
  Einstellungen) holt die Abfrage jederzeit zurück, z. B. bevor du den Rechner verlässt.
- Die Tabs (Titel, Dateipfade, Einstellungen je Tab) und alle Notizen werden **verschlüsselt** in
  `~/.local/share/quicknotes` gespeichert. Ohne Passwort sind sie nicht lesbar – auch nicht aus
  einer Kopie des Ordners. Allgemeine Einstellungen wie das Design bleiben lesbar, damit die
  Abfrage angezeigt werden kann.
- Von der Festplatte geöffnete Dateien bleiben dort unverändert (nur ungespeicherte Änderungen an
  ihnen werden verschlüsselt).
- *Ändern …* setzt ein neues Passwort, *Entfernen …* speichert wieder alles unverschlüsselt;
  beides fragt zuerst nach dem aktuellen Passwort.
- So funktioniert es: Ein zufälliger 256-Bit-Schlüssel verschlüsselt die Daten (XChaCha20-Poly1305,
  das auch jede Veränderung der Dateien erkennt). Dieser Schlüssel ist selbst mit einem Schlüssel
  verschlüsselt, der mit Argon2id (64 MiB, 3 Durchläufe) aus deinem Passwort abgeleitet wird –
  das macht das Erraten von Passwörtern langsam. Das Passwort selbst wird nie gespeichert.
- **Ein vergessenes Passwort lässt sich nicht wiederherstellen** – auch nicht vom Entwickler.
  Hinweis: Unverschlüsselte Kopien von vor dem Festlegen des Passworts können auf der Festplatte
  bleiben, bis sie überschrieben werden.

### Tab-Passwort

Einzelne Notizen können ein eigenes Passwort bekommen – mit oder ohne App-Passwort:

- *Mit Passwort schützen …* (Hauptmenü oder Rechtsklick auf den Tab) verschlüsselt die Notiz mit
  einem eigenen Passwort. Solange du sie lesen kannst, zeigt der Tab ein **offenes Schloss**.
- *Tab sperren* und jeder Start von QuickNotes verbergen den Inhalt: Der Tab zeigt ein
  **geschlossenes Schloss**, den Titel *Gesperrte Notiz* (außer du hast den Tab umbenannt, F2)
  und statt des Texts eine Passwortabfrage. Mit dem Passwort öffnest du ihn wieder.
- *Tab-Passwort ändern …* und *Tab-Passwort entfernen …* fragen zuerst nach dem aktuellen
  Passwort.
- Geschützte Notizen bleiben in Sicherungen, geschlossenen Tabs und auf der Festplatte
  (`.locked`-Dateien) verschlüsselt; nur ihr Passwort öffnet sie. Von der Festplatte geöffnete
  Dateien lassen sich nicht schützen – nur Notizen.
- Wie beim App-Passwort lässt sich ein vergessenes Tab-Passwort nicht wiederherstellen.

## Sicherung {#backup}

*Notizen sichern…* unter *Einstellungen → Import/Export* schreibt alle Tabs, Notizen und
Einstellungen in eine einzige Datei. *Sicherung wiederherstellen…* **fügt** die Tabs einer
Sicherung entweder zu den aktuellen **hinzu** oder **ersetzt** alles (der aktuelle Stand wird
vorher in einer automatischen Sicherung abgelegt).

- **Verschlüsselte Sicherungen:** Beim Sichern ein Passwort eingeben (mindestens 8 Zeichen,
  zweimal), um die Sicherung zu verschlüsseln (`.qnbackup`), oder *Ohne Passwort* für eine
  normale `.tar.gz`-Datei wählen. Beim Wiederherstellen fragt QuickNotes nach dem Passwort. Der
  Schlüssel wird mit Argon2id abgeleitet, die Daten werden mit XChaCha20-Poly1305 verschlüsselt
  und gegen Veränderung geschützt – ohne Passwort ist die Sicherung nicht lesbar, ein
  vergessenes Passwort lässt sich nicht wiederherstellen.

## Tastenkürzel {#shortcuts}

| Tastenkürzel | Aktion |
|---|---|
| Strg+N / Strg+T | Neue Notiz |
| Strg+Umschalt+N | Neue Berechnung |
<!-- if kanban -->
| Strg+Umschalt+K | Neues Kanban-Board |
<!-- end -->
| Strg+O, Strg+S, Strg+Umschalt+S | Öffnen, Speichern, Speichern unter |
| Strg+W, Strg+Umschalt+T | Tab schließen, geschlossenen Tab wiederherstellen |
| F2 | Tab umbenennen |
| F5 | Datei neu von der Festplatte laden |
| Strg+F, Strg+H, F3, Umschalt+F3 | Suchen, Ersetzen, Nächstes, Vorheriges |
<!-- if devtools -->
| Strg+Alt+B, Strg+Alt+M | Formatieren, Minimieren |
<!-- end -->
| Strg+Umschalt+V | Vorschau ein/aus |
| Strg+Umschalt+E, Strg+Umschalt+P | Als HTML, PDF exportieren |
<!-- if slides -->
| Strg+F5, Umschalt+F5 | Präsentation von Anfang an, ab aktueller Folie starten |
| Strg+Umschalt+F5 | In einem Fenster präsentieren |
<!-- end -->
| Strg+B, Strg+I, Strg+K | Markdown: fett, kursiv, Link |
| Strg+Enter | Checklisten-Eintrag abhaken, Zeilen zur Checkliste machen |
| Tab, Umschalt+Tab, Enter | Tabellen: nächste/vorige Zelle, nächste Zeile |
| Umschalt+Alt+Pfeiltasten | Tabellen: Zeile bzw. Spalte verschieben |
| Alt+Z | Zeilenumbruch ein/aus |
| Alt+Enter, Esc | Ablenkungsfreier Modus ein/aus, verlassen |
| Tab / Strg+E, Strg+Umschalt+G | Emmet-Abkürzung erweitern, mit Abkürzung umschließen (HTML, CSS, …) |
| Strg+, | Einstellungen |
| Strg+Mausrad, Strg +/−/0 | Schriftgröße |
| Strg+Tab, Strg+Bild↑/Bild↓, Alt+1…9 | Tab wechseln |
| F1 | Diese Hilfe |
<!-- if slides -->
| Umschalt+F1 | Syntax-Hilfe an der Schreibmarke (Präsentationen) |
<!-- end -->
| Strg+Q | Beenden |

[Zurück nach oben](#top)
