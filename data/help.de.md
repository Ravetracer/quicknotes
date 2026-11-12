# QuickNotes-Hilfe {#top}

**Inhalt:** [Notizen & Tabs](#notes) · [Checklisten](#checklists) · [Tabellen](#tables) · [Berechnungen](#calc) · [Vorlagen](#templates) ·
[Vorschau & Export](#previews) · [Präsentationen](#slides) · [Werkzeuge](#tools) · [Emmet](#emmet) · [Geocaching](#geocaching) ·
[Sicherung](#backup) ·
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
  Auch im Kontextmenü.
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
- Das Kontextmenü (Rechtsklick in eine Zelle) hat ein Untermenü **Tabelle**: formatieren,
  Zeilen und Spalten einfügen, verschieben und löschen, die Zeilen nach der Spalte
  **sortieren** (Zahlen numerisch, Text alphabetisch) und die Spalte links, zentriert oder
  rechts **ausrichten** (`:--`, `:-:`, `--:`).
- **Umschalt+Alt+↑/↓** verschieben die Zeile, **Umschalt+Alt+←/→** die Spalte.
- *Formatieren* (Strg+Alt+B) formatiert in einem Markdown-Dokument alle Tabellen auf einmal.

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

Markdown-, reStructuredText-, PlantUML- und HTML-Dateien lassen sich mit einer Live-Vorschau
neben dem Text anzeigen (Strg+Umschalt+V). Markdown und reStructuredText können über das
Hauptmenü oder das Kontextmenü des Editors (rechte Maustaste) als **HTML**
(Strg+Umschalt+E) oder **PDF** (Strg+Umschalt+P) exportiert werden.
Relative Bilder werden aus dem Ordner des Dokuments geladen.

PlantUML benötigt eine lokale PlantUML-Installation (Befehl `plantuml` oder eine
`plantuml*.jar` in `~/bin`).

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

## Werkzeuge {#tools}

Das Menü *Formatieren & Werkzeuge* (Kopfleiste) und das Kontextmenü des Editors (rechte
Maustaste) arbeiten auf der Auswahl oder, wenn nichts ausgewählt ist, auf dem ganzen Tab:

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

## Sicherung {#backup}

*Notizen sichern…* unter *Einstellungen → Import/Export* schreibt alle Tabs, Notizen und Einstellungen in eine einzige
`.tar.gz`-Datei. *Sicherung wiederherstellen…* **fügt** die Tabs einer Sicherung entweder zu den
aktuellen **hinzu** oder **ersetzt** alles (der aktuelle Stand wird vorher in einer
automatischen Sicherung abgelegt).

## Tastenkürzel {#shortcuts}

| Tastenkürzel | Aktion |
|---|---|
| Strg+N / Strg+T | Neue Notiz |
| Strg+Umschalt+N | Neue Berechnung |
| Strg+O, Strg+S, Strg+Umschalt+S | Öffnen, Speichern, Speichern unter |
| Strg+W, Strg+Umschalt+T | Tab schließen, geschlossenen Tab wiederherstellen |
| F2 | Tab umbenennen |
| F5 | Datei neu von der Festplatte laden |
| Strg+F, Strg+H, F3, Umschalt+F3 | Suchen, Ersetzen, Nächstes, Vorheriges |
| Strg+Alt+B, Strg+Alt+M | Formatieren, Minimieren |
| Strg+Umschalt+V | Vorschau ein/aus |
| Strg+Umschalt+E, Strg+Umschalt+P | Als HTML, PDF exportieren |
| Strg+F5, Umschalt+F5 | Präsentation von Anfang an, ab aktueller Folie starten |
| Strg+Umschalt+F5 | In einem Fenster präsentieren |
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
| Umschalt+F1 | Syntax-Hilfe an der Schreibmarke (Präsentationen) |
| Strg+Q | Beenden |

[Zurück nach oben](#top)
