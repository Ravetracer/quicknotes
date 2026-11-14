## 0.18.0 – 2026-11-14

- **Weitere Sync-Ziele**: Neben GitHub lassen sich Notizen mit Nextcloud (Anmeldung im Browser), jedem WebDAV-Server, S3-kompatiblem Speicher oder einem von einem anderen Programm synchronisierten Ordner (Dropbox, Syncthing, …) abgleichen – Ende-zu-Ende-verschlüsselt, beim Start, alle fünf Minuten und beim Beenden.
- **Datenformate** (abschaltbar): JSON, YAML und XML werden beim Tippen geprüft – Unterstreichungen, Tooltips und eine Problemliste (Strg+Umschalt+M); Stilregeln für YAML wie bei yamllint.
- **Schemas**: Prüfung und Vervollständigung mit JSON Schema und XML Schema (XSD) – der Notiz zugewiesen, im Dokument genannt oder über den Dateinamen im SchemaStore-Katalog gefunden.
- **Notizvorlagen**: Notizen im Ordner *Vorlagen* mit Twig-ähnlicher Syntax – Datums- und Zeitfelder, Fragen beim Einfügen, Schreibmarke, Auswahl und Zwischenablage, Bausteine (`include`) und Vererbung (`extends`/`block`). Neue Notizen aus Vorlagen (Strg+Alt+N) bleiben verbunden: spätere Änderungen der Vorlage werden übernommen, eigene Änderungen bleiben erhalten.
- **Spielleitung** (abschaltbar): Abenteuer mit Kapiteln, Charakterbögen (Vorlagen für D&D 5e, Pathfinder 2e, DSA 5), Würfeln (behalten/streichen, explodierend, Erfolgspools, Fate, DSA-Proben, eigene Würfel), Initiative, Spiel- und Weltzeit und Spielprotokoll.
- **Neuigkeiten** nach einem Update, die vollständige Änderungshistorie unter *Über QuickNotes* und eine Prüfung auf neue Versionen beim Start (in den Einstellungen abschaltbar).
- Behoben: JSON-Dateien mit `$schema` wurden als JavaScript geöffnet; ein Absturz, wenn eine neue Notiz mit bestimmten Umlauten begann.

## 0.17.0 – 2026-11-14

- **Mindmaps** (abschaltbar) aus Markdown-Gliederungen: Markdown in den Knoten, drei Layouts, Themen, Farben und eingeklappte Zweige je Knoten.
- Interaktive Vorschau: zoomen, verschieben, einklappen, Checkboxen abhaken, Zweige hinzufügen, Doppelklick springt zur Zeile.
- Export als SVG oder PDF.

## 0.16.0 – 2026-11-14

- **Obsidian-Vaults importieren** in einen Bibliotheksordner, mit Anhängen; ein erneuter Import fügt nur neue Notizen hinzu.
- Obsidian-Syntax in jeder Markdown-Vorschau: Wikilinks, Einbettungen, Hervorhebungen, Kommentare, Tags und Callouts.

## 0.15.0 – 2026-11-14

- **Einheiten in Rechenblättern**: Werte mit Einheiten, Umrechnungen (`in`, `nach`, `to`), eigene Einheiten, Zahlensysteme und römische Zahlen.
- **Währungen** mit den Tageskursen der Europäischen Zentralbank.
- **Daten, Zeitspannen, Zeitzonen und Kalender**, auch julianischer, hebräischer und islamischer Kalender.
- Dialog zum Umrechnen und Vervollständigung von Einheitennamen.

## 0.14.0 – 2026-11-14

- **Verschlüsselter Abgleich mit GitHub**: Notizen, Ordner und Papierkorb, Ende-zu-Ende-verschlüsselt mit einem Sync-Passwort; Änderungen auf beiden Seiten bleiben als Konfliktkopien erhalten.

## 0.13.0 – 2026-11-13

- **Notizbibliothek** mit verschachtelten Ordnern in einer Seitenleiste (F9): geschlossene Notizen bleiben in ihrem Ordner, dazu ein Papierkorb.

## 0.12.0 – 2026-11-13

- **Export als Bild** (PNG, JPEG, WebP) und **EPUB**.
- **Lernkarten** mit verteilter Wiederholung und Lernübersicht.
- Kontexthilfe: F1 öffnet die Hilfe beim Thema, an dem du gerade arbeitest.

## 0.11.0 – 2026-11-13

- **Abschaltbare Funktionen** in den Einstellungen, darunter ein Menü für Entwicklerwerkzeuge.
- Behoben: die Höhe von Kontextmenüs.

## 0.10.0 – 2026-11-13

- **Diagramme aus Markdown-Tabellen** (Balken, Linien, Torten und mehr).
- CSV-, Excel- und ODS-Daten als Tabellen importieren; als Tabelle einfügen.

## 0.9.0 – 2026-11-12

- **Formatierungsleiste für Markdown** und kontextabhängige Menüs.
- **LaTeX-Formeln** (KaTeX) und **ABC-Notenschrift** in Markdown.

## 0.8.0 – 2026-11-12

- **Checklisten**, **Tabellenbearbeitung** und **Kanban-Boards**.
- Wörter, Zeichen und Absätze in der Statusleiste.
- **App-Passwort**, **Tab-Passwörter** und passwortgeschützte Sicherungen.

## 0.7.0 – 2026-11-09

- **Emmet**-Abkürzungen für HTML, XML und CSS.

## 0.6.0 – 2026-11-09

- **Einstellungsdialog**, senkrechte Tabs, ablenkungsfreier Modus und 165 Farbthemen.
- Die Aufteilung zwischen Editor und Vorschau bleibt je Tab erhalten.

## 0.5.0 – 2026-11-06

- Unterstützung für **MongoDB / JSON5** und Umwandlung zwischen PHP-Arrays und Extended JSON.

## 0.4.0 – 2026-11-06

- Kontextmenüs, **Tab-Farben** und Syntaxhilfe für Präsentationen.

## 0.3.0 – 2026-11-06

- **Markdown-Präsentationen** mit Referentenansicht.

## 0.2.0 – 2026-11-05

- **Geocaching-Werkzeuge** (Chiffren, Koordinaten, Rätselhilfen).
- Deutsche Benutzeroberfläche.

## 0.1.0 – 2026-11-05

- Erste Version: ein Notizblock mit Tabs, automatischem Speichern und Syntaxhervorhebung.
- Vorschau für Markdown, reStructuredText, HTML und PlantUML; Export als HTML und PDF.
- **Rechenblätter** mit Vorlagen, Sicherung und Wiederherstellung, eingebaute Hilfe und ein AppImage.
