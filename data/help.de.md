# QuickNotes-Hilfe {#top}

**Inhalt:** [Notizen & Tabs](#notes) · [Berechnungen](#calc) · [Vorlagen](#templates) ·
[Vorschau & Export](#previews) · [Werkzeuge](#tools) · [Geocaching](#geocaching) · [Sicherung](#backup) ·
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
- Schriftart und -größe des Editors werden in der Statusleiste eingestellt (oder mit
  Strg+Mausrad).
- Die Sprache der Oberfläche (English, Deutsch oder Systemsprache) wird im Hauptmenü unter
  *Sprache* gewählt und gilt nach einem Neustart.

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
Hauptmenü als **HTML** (Strg+Umschalt+E) oder **PDF** (Strg+Umschalt+P) exportiert werden.
Relative Bilder werden aus dem Ordner des Dokuments geladen.

PlantUML benötigt eine lokale PlantUML-Installation (Befehl `plantuml` oder eine
`plantuml*.jar` in `~/bin`).

## Werkzeuge {#tools}

Das Menü *Formatieren & Werkzeuge* (Kopfleiste) arbeitet auf der Auswahl oder, wenn nichts
ausgewählt ist, auf dem ganzen Tab:

- **Formatieren / Minimieren** von JSON, XML, YAML und SQL (Strg+Alt+B / Strg+Alt+M erkennen
  das Format)
- **Umwandeln** JSON ↔ YAML
- **Zeilen:** sortieren, umkehren, Duplikate oder leere Zeilen entfernen, Leerzeichen am
  Zeilenende entfernen
- **Kodieren / Dekodieren:** Base64, URL, JSON-Strings
- **Groß-/Kleinschreibung:** GROSSBUCHSTABEN, kleinbuchstaben

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

*Notizen sichern…* im Hauptmenü schreibt alle Tabs, Notizen und Einstellungen in eine einzige
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
| Alt+Z | Zeilenumbruch ein/aus |
| Strg+Mausrad, Strg +/−/0 | Schriftgröße |
| Strg+Tab, Strg+Bild↑/Bild↓, Alt+1…9 | Tab wechseln |
| F1 | Diese Hilfe |
| Strg+Q | Beenden |

[Zurück nach oben](#top)
