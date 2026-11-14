# QuickNotes-Hilfe {#top}

**Inhalt:** [Notizen & Tabs](#notes) · [Ordner](#folders) · [Checklisten](#checklists) · [Tabellen](#tables) · [Formatierung](#formatting) · [Formeln & Noten](#notation) · [Diagramme](#charts) · [Kanban](#kanban) · [Mindmaps](#mindmaps) · [Lernkarten](#flashcards) · [Berechnungen](#calc) · [Vorlagen](#templates) ·
[Vorschau & Export](#previews) · [Präsentationen](#slides) · [Entwicklerwerkzeuge](#devtools) · [Emmet](#emmet) · [Geocaching](#geocaching) ·
[Sicherheit](#security) · [Sicherung](#backup) · [GitHub-Sync](#sync) ·
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
Kanban-Boards, Lernkarten, Notizordner, die Geocaching-Werkzeuge, PlantUML und die Entwicklerwerkzeuge. Ausgeschaltete Funktionen verschwinden
aus den Menüs, der Werkzeugleiste und dieser Hilfe; Dokumente bleiben, wie sie sind
(Präsentationen und Boards öffnen sich dann als Markdown, Diagramme erscheinen als Code). Neue
Installationen starten mit allen ausgeschaltet.

*Designs* enthält das dunkle und helle Standarddesign sowie 165 Designs aus Dracula, den
Dayle-Rees-Farbschemata und dem VSCode Ultimate Themes Pack. Ein Klick wendet ein Design sofort
auf Editor, Fenster und Vorschau an; Strg+F sucht in den Einstellungen nach Designnamen. Die Schriftgröße lässt sich auch
mit Strg+Mausrad oder Strg +/−/0 ändern.

## Ordner {#folders}

Die **Notizen-Seitenleiste** (Knopf oben links oder F9) hält alle Notizen in Ordnern – auch die,
die nicht geöffnet sind. Das Schließen eines Notiz-Tabs wirft die Notiz nicht mehr weg: Sie
bleibt in ihrem Ordner und öffnet sich mit einem Klick wieder. Geöffnete Notizen sind fett.

- **Neuer Ordner**: der Ordner-Knopf über dem Baum oder *Neuer Ordner* im Kontextmenü eines
  Ordners für einen Unterordner. Ordner lassen sich beliebig tief verschachteln und sind nach
  Namen sortiert.
- **Verschieben**: eine Notiz auf einen Ordner (oder auf *Ohne Ordner*) ziehen, einen Ordner in
  einen anderen ziehen. *Aus dem Ordner nehmen* im Kontextmenü holt eine Notiz heraus.
- **Neue Notiz** im Kontextmenü eines Ordners legt die Notiz direkt dort an.
- **Dateien**, die von der Festplatte geöffnet wurden, lassen sich ebenfalls in Ordner legen.
  Nach dem Schließen bleiben sie dort als Verknüpfung, die die Datei wieder öffnet; ein
  Warnzeichen markiert eine Datei, die es nicht mehr gibt. *Aus den Notizen entfernen* nimmt die
  Verknüpfung weg (die Datei selbst bleibt).
- **Löschen**: *In den Papierkorb* (oder auf den *Papierkorb* ziehen) legt eine Notiz in den
  Papierkorb; von dort lässt sie sich wiederherstellen oder endgültig löschen (*Papierkorb leeren*
  löscht alles darin). Beim Löschen eines Ordners bleiben seine Notizen und Unterordner erhalten
  – sie wandern eine Ebene nach oben.
- Leere Notizen ohne Titel werden beim Schließen weiterhin verworfen.

Ordner, geschlossene Notizen und der Papierkorb werden mit dem App-Passwort verschlüsselt und
sind in Sicherungen enthalten.


**Obsidian-Vault importieren**: *Hauptmenü → Obsidian-Vault importieren…* und den Ordner des
Vaults wählen. Seine Notizen werden mit allen Unterordnern in einen neuen, gleichnamigen Ordner
kopiert; der Vault selbst bleibt unverändert. Bilder und andere Dateien, die die Notizen
verwenden, werden mitkopiert (nach `~/.local/share/quicknotes/attachments`). Ein erneuter
Import desselben Vaults fügt nur Notizen hinzu, die noch nicht importiert wurden. Obsidians
eigene Syntax versteht die Vorschau, siehe [Formatierung](#formatting).

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

**Obsidian-Syntax** funktioniert in jeder Markdown-Vorschau:

| Syntax | Ergebnis |
|---|---|
| `[[Notiz]]`, `[[Notiz\|Text]]`, `[[Notiz#Überschrift]]` | Verweis auf eine Notiz; ein Klick öffnet sie |
| `![[bild.png]]`, `![[bild.png\|300]]` | eingebettetes Bild (optional 300 px breit) |
| `![[Notiz]]`, `![[datei.pdf]]` | Verweis auf die Notiz oder Datei |
| `==markiert==` | hervorgehobener Text |
| `%%Kommentar%%` | in der Vorschau verborgen (auch über mehrere Zeilen) |
| `#tag` | als Schlagwort dargestellt |
| `> [!warning] Titel` | Hinweis-Box (note, tip, warning, danger, example, quote, …); `[!tip]-` startet zugeklappt |

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

## Lernkarten {#flashcards}

**Lernkarten** helfen beim Lernen mit verteilter Wiederholung: Jede Karte kommt wieder, kurz
bevor man sie vergessen würde. *Hauptmenü → Neue Lernkarten* legt ein Dokument mit Beispielen
an. Ein Markdown-Dokument ist ein Satz Lernkarten, wenn sein Front Matter `flashcards: true`
enthält (oder die Datei auf `.cards.md` endet).

    ---
    flashcards: true
    deck: Biologie/Zellen
    ---

    # Grundlagen

    Hauptstadt von Frankreich::Paris
    Mitochondrien:::Kraftwerk der Zelle

    Was ist Osmose?
    ?
    Wasser wandert durch eine Membran
    von niedriger zu hoher Konzentration.

    Was sind Organellen?
    ?
    - [x] Mitochondrium
    - [x] Ribosom
    - [ ] Zellwand
    Ribosomen haben keine Membran.

    Der größte Planet?
    ?
    - ( ) Mars
    - (x) Jupiter

- `Frage::Antwort` – eine Karte in einer Zeile.
- `Frage:::Antwort` – zwei Karten: Die Frage wird auch umgekehrt gestellt.
- Zeilen der Frage, eine Zeile nur mit `?`, dann die Zeilen der Antwort – eine Karte über
  mehrere Zeilen (`??` für beide Richtungen). Eine Leerzeile beendet die Karte. Antworten
  können alles aus Markdown nutzen: Listen, Code, Formeln, Bilder.
- Eine Antwort aus `- [x]`-/`- [ ]`-Punkten ist **Multiple Choice mit Kontrollkästchen**
  (beliebig viele richtige Antworten), aus `- (x)`-/`- ( )`-Punkten **Multiple Choice mit
  Optionsfeldern** (genau eine richtige Antwort). Zeilen nach den Punkten erklären die Antwort.

**Stapel:** `deck: Biologie/Zellen` im Front Matter legt die Karten in einen Stapel und seinen
Unterstapel (beliebig tief; mehrere Dokumente können denselben Stapel füllen). Überschriften
im Dokument bilden weitere Unterstapel. Ohne `deck:` ist der Titel (oder der Dateiname) der
Stapel.

**Lernen:** Die Vorschau zeigt die Stapel mit Fortschritt und Statistik. **Lernen** öffnet die
fälligen Karten und neue (höchstens 20 neue am Tag, `new-per-day:` ändert das); **Üben** geht
alle Karten eines Stapels in zufälliger Reihenfolge durch, ohne den Plan zu ändern. Im
Lernfenster:

| Taste | Aktion |
|---|---|
| Leertaste, Enter, Klick | Antwort zeigen (Multiple Choice: Auswahl prüfen) |
| 1–9 | Multiple Choice: Antwort auswählen |
| 1 – 4 | Nochmal, Schwer, Gut, Leicht (Leertaste: Gut) |
| F11 | Vollbild |
| Esc | Beenden |

Jeder Knopf zeigt, wann die Karte wiederkommt. *Nochmal* wiederholt die Karte in derselben
Runde. Falsch beantwortete Karten lassen sich am Ende noch einmal üben. *Hauptmenü →
Lernübersicht* zeigt die Stapel aller geöffneten Lernkarten-Dokumente zusammen.

Der **Fortschritt** wird im Datenordner von QuickNotes gespeichert, nicht im Dokument –
verschlüsselt, solange das App-Passwort an ist, und in Sicherungen enthalten. Eine Karte behält
ihren Fortschritt, wenn ihre Frage leicht geändert wird. Die Statistik zählt Antworten,
richtige Antworten, die Lernzeit und die Tage in Folge.

**Aussehen:** `theme:` im Front Matter wählt `light`, `dark`, `sepia`, `night` oder `contrast`
(Standard: wie QuickNotes). `accent:`, `background:` und `color:` setzen Farben (`#rrggbb`);
schwer lesbare Textfarben werden automatisch angepasst.

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

## Mindmaps {#mindmaps}

Eine **Mindmap** entsteht aus einer Markdown-Gliederung. Neu anlegen mit *Neue Mindmap* im
Hauptmenü, oder eine Markdown-Datei mit dem Front Matter `mindmap: 1` (oder der Endung
`.mindmap.md`) versehen.

```
---
mindmap: 1
theme: pastel      # default, pastel, vivid, mono, dark, ocean, forest
layout: both       # both, right, down
font: Cantarell    # optional
---
# Urlaub 2027
- Reiseziel {color: #e01b24}
  - **Portugal**
  - Spanien
- Budget
  - Flüge ~ 600 €
- Packliste {folded}
  - [x] Reisepass
  - [ ] Sonnencreme
```

- Die erste `#`-Überschrift ist die Mitte. Verschachtelte Listenpunkte (`-`, `*`, `+`, `1.`)
  sind die Äste; `##`-, `###`-… Überschriften funktionieren ebenfalls als Äste, mit Listen
  darunter. Eine Zeile ohne Markierung setzt den Knoten darüber fort.
- Der Text behält seine Markdown-Formatierung: **fett**, *kursiv*, `Code`, ~~durchgestrichen~~
  und Links; `[ ]` / `[x]` zeigen ein Kästchen. Langer Text wird umbrochen.
- Am Zeilenende färbt `{color: #e01b24}` (oder kurz `{#e01b24}`) einen Ast samt allem darunter,
  `{folded}` lässt ihn zugeklappt beginnen.
- **Anordnung**: `both` verteilt die Äste links und rechts der Mitte, `right` lässt einen Baum
  nach rechts wachsen, `down` zeichnet ein Organigramm.
- **Themes** bestimmen die Farben; `default` folgt dem hellen oder dunklen Aussehen von
  QuickNotes. Eigene Astfarben: `colors: ['#e01b24', '#3584e4']`.

In der Vorschau zoomt **Strg+Mausrad** (oder `+`, `−`), das Mausrad oder Ziehen verschiebt die
Karte, `0` oder der Knopf unten passt sie wieder ein. Der kleine Knopf neben einem Ast klappt ihn
zu und auf, ein **Doppelklick** auf einen Knoten springt zu seiner Zeile. Ein Klick auf ein
Kästchen hakt die Aufgabe ab (auch im Text). Das **+** neben einem Knoten (sichtbar, wenn die Maus
darüber ist) fügt einen Ast hinzu: Text eingeben und Enter drücken – das Feld bleibt für den
nächsten offen, Esc schließt es. Die neue Zeile wird wie ihre Geschwister geschrieben (gleiche
Einrückung und Listenmarkierung).

**Export** (Hauptmenü oder Kontextmenü): *Als PDF exportieren…* setzt die Mindmap auf eine
A4-Seite – quer oder hoch, je nachdem, was besser passt – und *Als SVG exportieren…* speichert
sie als Vektorgrafik. Exporte nutzen beim Theme `default` das helle Aussehen.

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

## Einheiten und Umrechnungen {#units}

Zahlen in einer Berechnung können **Einheiten** haben. Das Ergebnis behält die Einheit des
ersten Werts; `to` (auch `in`, `as`, `nach`, `->`) rechnet um:

```
a = 3 m + 20 cm          # = 3.2 m
a to ft                  # = 10.4987 ft
tempo = 120 km / 1.5 h   # = 80 km/h
tempo in m/s             # = 22.2222 m/s
100 °F to °C             # = 37.7778 °C
2 h 30 min to min        # = 150 min
100 min to h min         # = 1 h 40 min
6 L/100km to mpg         # = 39.2024 mpg
250 EUR in USD           # = 270.50 USD (Tageskurs)
```

- Einheiten: Länge (`m`, `km`, `in`, `ft`, `mi`, `nmi`, …), Fläche (`m²`, `ha`, `ac`), Volumen
  (`L`, `mL`, `gal`, `cup`, `EL`, `TL`, …), Masse (`g`, `kg`, `t`, `lb`, `oz`, `ct`, `Pfund`),
  Zeit (`s`, `min`, `h`, `d`, `wk`, `month`, `yr`), Geschwindigkeit (`km/h`, `mph`, `kn`),
  Temperatur (`°C`, `°F`, `K`), Druck (`Pa`, `hPa`, `bar`, `psi`, `atm`), Energie (`J`, `kWh`,
  `kcal`, `eV`), Leistung (`W`, `kW`, `PS`, `hp`), Kraft (`N`, `kp`), Winkel (`°`, `rad`,
  `gon`), Daten (`bit`, `B`, `kB`, `MB`, `GiB`, `Mbps`), Frequenz (`Hz`, `rpm`), Elektrizität
  (`A`, `V`, `Ω`, `mAh`) und Währungen (`EUR`, `USD`, `GBP`, `CHF`, `JPY`, `€`, `$`, `£`, …).
- Übliche Vorsätze funktionieren (`km`, `mg`, `kWh`, `MiB`), ebenso ausgeschriebene Namen
  (`Meter`, `Stunden`), `m²` oder `m2` und Kombinationen wie `kg*m/s^2`. **Beim Tippen** werden
  Einheiten und Umrechnungsziele vorgeschlagen.
- Mehrere Einheiten auf einmal: `5 ft 3 in to cm`, `to h min`, `to d h min`.
- Werte verschiedener Art lassen sich nicht addieren: `5 kg + 3 m` ist ein Fehler. Reine
  Zahlen lassen sich nach `hex`, `bin`, `oct` und `roman` umrechnen (`255 to hex` = 0xFF).
- Eine Zahl direkt vor einer Einheit oder einem Namen bindet stärker als `/`:
  `120 km / 1.5 h` teilt durch 1,5 Stunden. Variablen haben Vorrang vor gleichnamigen Einheiten.
- Temperaturen: `25 °C - 20 °C` ist eine Differenz (5 °C = 5 K), `20 °C + 5 K` = 25 °C.
- Eigene Einheiten: `unit etage = 2.8 m`, dann `5 etage to m`.
- **Währungen** nutzen die täglichen Referenzkurse der Europäischen Zentralbank. Sie werden nur
  geladen, wenn ein Rechenblatt eine andere Währung als den Euro verwendet, und für die
  Offline-Nutzung gespeichert.

Der **Einheitenumrechner** (Knopf in der Kopfleiste oder *Einheitenumrechner…* im Kontextmenü)
lässt Art, Einheiten und Wert auswählen, zeigt das Ergebnis und fügt die Zeile ins
Rechenblatt ein.

**Datum und Uhrzeit**

```
2026-12-24 - heute             # Tage bis Weihnachten
heute + 6 weeks                # ein Datum
24.12.2026 to weekday          # Donnerstag
2026-01-31 + 1 month           # Sa. 28.02.2026
14:00 Berlin to Tokyo          # 22:00 JST
jetzt in New York              # aktuelle Uhrzeit dort
2026-12-24 18:00 - jetzt       # z. B. 40 d 2 h 37 min
1700000000 to date             # Unix-Zeitstempel → Datum
```

- Datumsangaben als `2026-12-24` oder `24.12.2026`, Uhrzeiten als `14:30` oder `14:30:15`,
  beides zusammen als `2026-12-24 18:00`. `heute`, `jetzt`, `morgen` und `gestern` gehen auch
  (ebenso `today`, `now`, `tomorrow`, `yesterday`). Achtung: `14:30` ohne Leerzeichen ist eine
  Uhrzeit; zum Teilen `14 : 30` oder `14 / 30` schreiben.
- Ganze Tage, Wochen, Monate und Jahre zählen im Kalender; Stunden und Minuten ergeben eine
  Uhrzeit.
- Zeitzonen: Städtenamen (`Tokyo`, `New York`, `München`), IANA-Namen (`Europe/Berlin`),
  Abkürzungen (`MEZ`, `CET`, `PST`), `UTC`, `UTC+2` oder `local`. Eine Zone nach einer Uhrzeit
  oder einem Datum sagt, wo sie gilt; nach `to` wird umgerechnet.
- Datumsziele: `weekday`, `week` (ISO-Kalenderwoche), `dayofyear`, `unix`, `jd` (Julianisches
  Datum), `julian`, `hebrew`, `hijri` (arithmetischer islamischer Kalender, kann um einen Tag
  vom beobachteten abweichen), `iso`.
- Funktionen: `date(j, m, t)`, `weekday(d)`, `week(d)`, `year(d)`, `month(d)`, `day(d)`,
  `unix(sekunden)`, `jd(n)` sowie aus anderen Kalendern `julian(j, m, t)`, `hebrew(j, m, t)`
  (Monat 1 = Nisan, 7 = Tischri) und `hijri(j, m, t)`.

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
neben dem Text anzeigen (Strg+Umschalt+V). Markdown und reStructuredText lassen sich über das
Hauptmenü oder das Kontextmenü des Editors (rechte Maustaste) exportieren als:

- **HTML** (Strg+Umschalt+E): eine einzelne Datei, in der alles steckt, auch Formeln und
  Diagramme.
- **PDF** (Strg+Umschalt+P): A4-Seiten.
- **Bild** (SVG, PNG oder JPG): das ganze Dokument als **ein langes Bild** oder **ein Bild pro
  Seite** wie im PDF (nummerierte Dateien in einem Ordner nach Wahl). SVG bleibt in jeder Größe
  scharf; PNG und JPG sind Pixelbilder in hoher Auflösung.
- **EPUB**: ein E-Book für E-Reader und Lese-Apps. Ein neues Kapitel beginnt an jeder
  `#`-Überschrift (an jeder `##`-Überschrift, wenn es nur eine `#` gibt); das
  Inhaltsverzeichnis zeigt die Kapitel und ihre Abschnitte. Formeln, Diagramme, Notenschrift
  und die Bilder des Dokuments kommen mit. Im Front Matter lassen sich die Angaben zum Buch
  festlegen:

      ---
      title: Mein Buch
      author: Erika Mustermann
      language: de
      date: 2026-11-13
      description: Worum es im Buch geht
      publisher: Eigenverlag
      cover: bilder/titel.jpg
      ---

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
pro Folie mit allen Schritten, *Als Bild exportieren* ein Bild pro Folie (SVG, PNG oder JPG).

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

### MongoDB-Abfragen und JSON5 {#mongodb}

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

## GitHub-Sync {#sync}

Hält deine Notizen über ein GitHub-Repository auf mehreren Rechnern gleich –
*Einstellungen → Sync*:

1. **Anmelden**: *Anmelden …* zeigt einen Code; *Code kopieren und GitHub öffnen*, auf
   github.com einfügen und bestätigen. Alternativ *Token verwenden …* mit einem
   Fine-grained-Zugriffstoken, das Zugriff auf das Repository und die Berechtigung
   *Contents: Read and write* hat.
2. **Repository**: Lege auf GitHub ein Repository an (am besten ein privates), gib es als
   `Besitzer/Name` ein und drücke Enter. Beim ersten Mal wählst du ein **Sync-Passwort**; jeder
   weitere Rechner fragt einmalig danach.
3. Ab dann holt QuickNotes Änderungen beim Start und sendet sie beim Schließen
   (*Automatisch abgleichen*). *Hauptmenü → Mit GitHub abgleichen* gleicht sofort ab.

- **Was abgeglichen wird**: alle Notizen – offene und die in der Bibliothek – mit Titel, Syntax
  und Ordner, die Ordner und der Papierkorb. Von der Festplatte geöffnete Dateien nicht.
  Passwortgeschützte Notizen reisen mit ihrem eigenen Passwort verschlüsselt; ist eine gerade
  entsperrt, wartet sie, bis sie wieder gesperrt ist.
- **Alles wird verschlüsselt**, bevor es den Rechner verlässt – in privaten wie in öffentlichen
  Repositorys: GitHub sieht nur verschlüsselte Dateien im Ordner `quicknotes/` (und wie viele es
  sind und wann sie sich ändern). Der Schlüssel ist zufällig und durch das Sync-Passwort
  geschützt (Argon2id, XChaCha20-Poly1305). Schlüssel und GitHub-Token liegen im Schlüsselbund
  (Secret Service), nie in den Einstellungen.
- **Konflikte**: Wurde eine Notiz zwischen zwei Abgleichen auf zwei Rechnern geändert, bleiben
  beide Fassungen erhalten – die andere als Kopie mit „Konflikt“ und der Uhrzeit im Titel.
  Umbenennen, in einen anderen Ordner verschieben und den Inhalt bearbeiten werden getrennt
  zusammengeführt und geraten daher nicht in Konflikt.
- **Endgültig löschen** (aus dem Papierkorb) löscht eine Notiz auch auf den anderen Rechnern,
  außer sie wurde dort inzwischen geändert.
- Jeder Abgleich ist ein Commit; die Historie des Repositorys enthält also ältere
  (verschlüsselte) Fassungen.
- *Trennen* beendet den Abgleich mit dem Repository (seine Daten bleiben auf GitHub);
  *Abmelden* vergisst zusätzlich das Token. Der Sync braucht die [Notizordner](#folders).
- Ein vergessenes Sync-Passwort lässt sich nicht wiederherstellen; die Notizen auf deinen
  Rechnern sind davon nicht betroffen.

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
<!-- if folders -->
| F9 | Notizen-Seitenleiste mit Ordnern |
<!-- end -->
<!-- if slides -->
| Umschalt+F1 | Syntax-Hilfe an der Schreibmarke (Präsentationen) |
<!-- end -->
| Strg+Q | Beenden |

[Zurück nach oben](#top)
