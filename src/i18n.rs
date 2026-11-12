//! User interface translations (English, German).
//!
//! English strings are the keys: `t!("Save")` returns the translation for
//! the active language, `t!("Saved {}", path)` fills `{}` placeholders in
//! order. The language is chosen once at startup.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    De,
}

static LANG: AtomicU8 = AtomicU8::new(0);

/// Sets the language from the setting: "en", "de" or anything else for the
/// system language (`LC_ALL`, `LC_MESSAGES`, `LANG`).
pub fn init(setting: &str) {
    let lang = match setting {
        "en" => Lang::En,
        "de" => Lang::De,
        _ => system_lang(),
    };
    LANG.store(lang as u8, Ordering::Relaxed);
}

fn system_lang() -> Lang {
    let var = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .unwrap_or_default();
    if var.starts_with("de") {
        Lang::De
    } else {
        Lang::En
    }
}

pub fn lang() -> Lang {
    if LANG.load(Ordering::Relaxed) == Lang::De as u8 {
        Lang::De
    } else {
        Lang::En
    }
}

fn german() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| DE.iter().copied().collect())
}

/// The translation of `s` (or `s` itself when there is none).
pub fn tr(s: &'static str) -> &'static str {
    match lang() {
        Lang::En => s,
        Lang::De => german().get(s).copied().unwrap_or(s),
    }
}

/// Replaces the `{}` placeholders of `template` in order.
pub fn fill(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut args = args.iter();
    let mut rest = template;
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        if let Some(a) = args.next() {
            out.push_str(&a.to_string());
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// `t!("text")` → `&'static str`, `t!("text {}", arg)` → `String`.
macro_rules! t {
    ($s:literal) => {
        $crate::i18n::tr($s)
    };
    ($s:literal, $($a:expr),+ $(,)?) => {
        $crate::i18n::fill($crate::i18n::tr($s), &[$(&$a as &dyn std::fmt::Display),+])
    };
}

const DE: &[(&str, &str)] = &[
    ("New note (Ctrl+N)", "Neue Notiz (Strg+N)"),
    ("Open file (Ctrl+O)", "Datei öffnen (Strg+O)"),
    ("Save to file (Ctrl+S)", "In Datei speichern (Strg+S)"),
    ("Main menu", "Hauptmenü"),
    ("Format & tools", "Formatieren & Werkzeuge"),
    ("Toggle preview (Ctrl+Shift+V)", "Vorschau umschalten (Strg+Umschalt+V)"),
    ("Find (Ctrl+F)", "Suchen (Strg+F)"),
    ("Rename…", "Umbenennen…"),
    ("Copy Path", "Pfad kopieren"),
    ("Close Other Tabs", "Andere Tabs schließen"),
    ("Close", "Schließen"),
    ("Font size – Ctrl+wheel or Ctrl+/−, click to reset", "Schriftgröße – Strg+Mausrad oder Strg+/−, Klick setzt zurück"),
    ("Plain Text", "Reiner Text"),
    ("Syntax", "Syntax"),
    ("No Open Notes", "Keine offenen Notizen"),
    ("Create a note with Ctrl+N or open a file with Ctrl+O.\nFiles can also be dropped here.", "Mit Strg+N eine Notiz anlegen oder mit Strg+O eine Datei öffnen.\nDateien können auch hierher gezogen werden."),
    ("New Note", "Neue Notiz"),
    ("Path copied", "Pfad kopiert"),
    ("This note has no file", "Diese Notiz hat keine Datei"),
    ("A small tabbed notepad for text and code snippets.\nNotes are stored in ~/.local/share/quicknotes.", "Ein kleiner Notizblock mit Tabs für Texte und Code-Schnipsel.\nNotizen werden in ~/.local/share/quicknotes gespeichert."),
    ("Could not read the dropped files", "Die abgelegten Dateien konnten nicht gelesen werden"),
    ("Could not read the dropped text", "Der abgelegte Text konnte nicht gelesen werden"),
    ("Note – stored automatically", "Notiz – wird automatisch gespeichert"),
    ("Reopen Closed Tab", "Geschlossenen Tab wiederherstellen"),
    ("“{}” closed", "„{}“ geschlossen"),
    ("Undo", "Rückgängig"),
    ("Save changes to “{}”?", "Änderungen an „{}“ speichern?"),
    ("The file has unsaved changes. They will be lost if you don't save them.", "Die Datei hat ungespeicherte Änderungen. Sie gehen verloren, wenn sie nicht gespeichert werden."),
    ("_Cancel", "_Abbrechen"),
    ("_Discard", "_Verwerfen"),
    ("_Save", "_Speichern"),
    ("Save failed: {}", "Speichern fehlgeschlagen: {}"),
    ("No closed tabs", "Keine geschlossenen Tabs"),
    ("Cannot open {}: {}", "{} kann nicht geöffnet werden: {}"),
    ("File not found: {}", "Datei nicht gefunden: {}"),
    ("Cannot store note: {}", "Notiz kann nicht gespeichert werden: {}"),
    ("Back Up Notes", "Notizen sichern"),
    ("Backup of {} saved to {}", "Sicherung von {} gespeichert unter {}"),
    ("Backup failed: {}", "Sicherung fehlgeschlagen: {}"),
    ("QuickNotes backups", "QuickNotes-Sicherungen"),
    ("Restore Backup", "Sicherung wiederherstellen"),
    ("Restore failed: {}", "Wiederherstellen fehlgeschlagen: {}"),
    ("Restore Backup?", "Sicherung wiederherstellen?"),
    ("Backup from {} with {}.\n\nAdd: puts the tabs of the backup next to your current tabs.\n\nReplace: closes all current tabs and restores the backup including its settings. Your current notes are saved to an automatic backup first.", "Sicherung vom {} mit {}.\n\nHinzufügen: öffnet die Tabs der Sicherung zusätzlich zu den aktuellen Tabs.\n\nErsetzen: schließt alle aktuellen Tabs und stellt die Sicherung samt Einstellungen wieder her. Die aktuellen Notizen werden vorher automatisch gesichert."),
    ("_Add", "_Hinzufügen"),
    ("_Replace", "_Ersetzen"),
    ("All tabs of the backup are already open", "Alle Tabs der Sicherung sind bereits geöffnet"),
    ("Added {} from the backup", "{} aus der Sicherung hinzugefügt"),
    (" – {} not found", " – {} nicht gefunden"),
    ("Restore cancelled, the current notes could not be saved: {}", "Wiederherstellen abgebrochen, die aktuellen Notizen konnten nicht gesichert werden: {}"),
    ("Restored {}. Previous notes saved to {}", "{} wiederhergestellt. Bisherige Notizen gesichert unter {}"),
    ("Not a file: {}", "Keine Datei: {}"),
    ("Large file – syntax highlighting and word wrap are off", "Große Datei – Syntaxhervorhebung und Zeilenumbruch sind aus"),
    ("Open Files", "Dateien öffnen"),
    ("Saved {}", "{} gespeichert"),
    ("Export works for Markdown and reStructuredText", "Export ist für Markdown und reStructuredText möglich"),
    ("Export as {}", "Als {} exportieren"),
    ("Exporting PDF…", "PDF wird exportiert…"),
    ("Exported to {}", "Exportiert nach {}"),
    ("Open", "Öffnen"),
    ("Export failed: {}", "Export fehlgeschlagen: {}"),
    ("Save As", "Speichern unter"),
    ("This note has no file to reload", "Diese Notiz hat keine Datei zum Neuladen"),
    ("Discard Changes and Reload?", "Änderungen verwerfen und neu laden?"),
    ("Your unsaved changes to this file will be lost.", "Die ungespeicherten Änderungen an dieser Datei gehen verloren."),
    ("_Reload", "_Neu laden"),
    ("Only text tabs can be renamed", "Nur Text-Tabs können umbenannt werden"),
    ("Rename Tab", "Tab umbenennen"),
    ("Leave empty to name the tab after its first line or file.", "Leer lassen, um den Tab nach seiner ersten Zeile oder Datei zu benennen."),
    ("_Rename", "_Umbenennen"),
    ("Nothing to change", "Nichts zu ändern"),
    ("Select some text first", "Bitte zuerst Text markieren"),
    ("_Copy", "_Kopieren"),
    ("_Insert Below", "_Darunter einfügen"),
    ("Result", "Ergebnis"),
    ("Result copied", "Ergebnis kopiert"),
    ("Note", "Notiz"),
    ("{} lines", "{} Zeilen"),
    ("large file, highlighting off", "große Datei, Hervorhebung aus"),
    ("Ln {}, Col {}", "Z. {}, Sp. {}"),
    ("{} words  ·  {} characters  ·  {} paragraphs", "{} Wörter  ·  {} Zeichen  ·  {} Absätze"),
    ("Selection: {}", "Auswahl: {}"),
    ("Replaced {} occurrences", "{} Treffer ersetzt"),
    ("Nothing replaced", "Nichts ersetzt"),
    ("Replace failed: {}", "Ersetzen fehlgeschlagen: {}"),
    ("No matches", "Keine Treffer"),
    ("{} of {}", "{} von {}"),
    ("{} matches", "{} Treffer"),
    ("Monospace fonts", "Schriften mit fester Breite"),
    ("Other fonts", "Andere Schriften"),
    ("Editor font", "Editor-Schrift"),
    ("Find", "Suchen"),
    ("Replace with", "Ersetzen durch"),
    ("Match case", "Groß-/Kleinschreibung beachten"),
    ("Whole words", "Ganze Wörter"),
    ("Regular expression", "Regulärer Ausdruck"),
    ("Replace", "Ersetzen"),
    ("Replace current match (Enter)", "Aktuellen Treffer ersetzen (Eingabetaste)"),
    ("Replace All", "Alle ersetzen"),
    ("Replace all matches", "Alle Treffer ersetzen"),
    ("New Calculation", "Neue Berechnung"),
    ("Open File…", "Datei öffnen…"),
    ("Save", "Speichern"),
    ("Save As…", "Speichern unter…"),
    ("Reload from Disk", "Neu von der Festplatte laden"),
    ("Export as HTML…", "Als HTML exportieren…"),
    ("Export as PDF…", "Als PDF exportieren…"),
    ("Find…", "Suchen…"),
    ("Find and Replace…", "Suchen und Ersetzen…"),
    ("Rename Tab…", "Tab umbenennen…"),
    ("Word Wrap", "Zeilenumbruch"),
    ("Font Ligatures", "Schrift-Ligaturen"),
    ("Smaller font (Ctrl+−)", "Kleinere Schrift (Strg+−)"),
    ("Larger font (Ctrl++)", "Größere Schrift (Strg++)"),
    ("Back Up Notes…", "Notizen sichern…"),
    ("Restore Backup…", "Sicherung wiederherstellen…"),
    ("Open Notes Folder", "Notizen-Ordner öffnen"),
    ("System Default", "Systemsprache"),
    ("Language", "Sprache"),
    ("Help", "Hilfe"),
    ("About QuickNotes", "Über QuickNotes"),
    ("Applies to the selection or the whole tab", "Wirkt auf die Auswahl oder den ganzen Tab"),
    ("Beautify (auto-detect)", "Formatieren (automatisch erkannt)"),
    ("Minify (auto-detect)", "Minimieren (automatisch erkannt)"),
    ("Format", "Formatieren"),
    ("Pretty JSON", "JSON formatieren"),
    ("Minify JSON", "JSON minimieren"),
    ("Pretty MongoDB / JSON5", "MongoDB / JSON5 formatieren"),
    ("Minify MongoDB / JSON5", "MongoDB / JSON5 minimieren"),
    ("mongosh → Extended JSON (canonical)", "mongosh → Extended JSON (kanonisch)"),
    ("PHP array → Extended JSON", "PHP-Array → Extended JSON"),
    ("PHP array → mongosh", "PHP-Array → mongosh"),
    ("Extended JSON / mongosh → PHP array", "Extended JSON / mongosh → PHP-Array"),
    ("No PHP array found.", "Kein PHP-Array gefunden."),
    ("Can't convert \"{}\" to PHP", "\"{}\" lässt sich nicht in PHP umwandeln"),
    ("Pretty XML", "XML formatieren"),
    ("Minify XML", "XML minimieren"),
    ("Pretty YAML", "YAML formatieren"),
    ("Pretty SQL", "SQL formatieren"),
    ("Convert", "Umwandeln"),
    ("Lines", "Zeilen"),
    ("Sort", "Sortieren"),
    ("Sort (case-insensitive)", "Sortieren (Groß-/Kleinschreibung ignorieren)"),
    ("Reverse", "Umkehren"),
    ("Remove Duplicates", "Duplikate entfernen"),
    ("Remove Empty Lines", "Leere Zeilen entfernen"),
    ("Trim Trailing Whitespace", "Leerzeichen am Zeilenende entfernen"),
    ("Encode / Decode", "Kodieren / Dekodieren"),
    ("Base64 Encode", "Base64 kodieren"),
    ("Base64 Decode", "Base64 dekodieren"),
    ("URL Encode", "URL kodieren"),
    ("URL Decode", "URL dekodieren"),
    ("JSON String Escape", "JSON-String maskieren"),
    ("JSON String Unescape", "JSON-String demaskieren"),
    ("Case", "Groß-/Kleinschreibung"),
    ("UPPERCASE", "GROSSBUCHSTABEN"),
    ("lowercase", "kleinbuchstaben"),
    ("{} tab", "{} Tab"),
    ("{} tabs", "{} Tabs"),
    ("{} file", "{} Datei"),
    ("{} files", "{} Dateien"),
    ("{} snippet", "{}-Schnipsel"),
    ("Untitled", "Unbenannt"),
    ("Copied “{}”", "„{}“ kopiert"),
    ("Fit", "Eingepasst"),
    ("Zoom out", "Verkleinern"),
    ("Zoom in", "Vergrößern"),
    ("Original size", "Originalgröße"),
    ("Fit to window", "An Fenster anpassen"),
    ("Image", "Bild"),
    ("Minify works for JSON, MongoDB / JSON5 and XML only.", "Minimieren ist nur für JSON, MongoDB / JSON5 und XML möglich."),
    ("Unterminated string in line {}", "Nicht geschlossene Zeichenkette in Zeile {}"),
    ("Unterminated comment in line {}", "Nicht geschlossener Kommentar in Zeile {}"),
    ("Unterminated regular expression in line {}", "Nicht geschlossener regulärer Ausdruck in Zeile {}"),
    ("Unbalanced \"{}\" in line {}", "Unpassendes \"{}\" in Zeile {}"),
    ("Unexpected \"{}\" in line {}", "Unerwartetes \"{}\" in Zeile {}"),
    ("Unexpected end of input", "Unerwartetes Ende der Eingabe"),
    ("Invalid {}: {}", "Ungültiger Wert für {}: {}"),
    ("Can't convert \"{}\" to Extended JSON", "\"{}\" lässt sich nicht in Extended JSON umwandeln"),
    ("{}() needs a value to be converted", "{}() braucht zum Umwandeln einen Wert"),
    ("Select the document or pipeline to convert.", "Bitte das Dokument oder die Pipeline zum Umwandeln auswählen."),
    ("URL decode: {}", "URL-Dekodierung: {}"),
    ("Invalid JSON: {}", "Ungültiges JSON: {}"),
    ("Invalid XML at byte {}: {}", "Ungültiges XML bei Byte {}: {}"),
    ("Invalid XML: unclosed element", "Ungültiges XML: nicht geschlossenes Element"),
    ("Invalid YAML: {}", "Ungültiges YAML: {}"),
    ("Not valid Base64", "Kein gültiges Base64"),
    ("Decoded data is not UTF-8 text", "Die dekodierten Daten sind kein UTF-8-Text"),
    ("Invalid JSON string: {}", "Ungültiger JSON-String: {}"),
    ("invalid format \":{}\" (use e.g. :3, :.2 or :3.2)", "ungültiges Format \":{}\" (z. B. :3, :.2 oder :3.2)"),
    ("invalid number \"{}\"", "ungültige Zahl \"{}\""),
    ("unexpected \"{}\"", "unerwartetes \"{}\""),
    ("empty expression", "leerer Ausdruck"),
    ("unexpected {}", "unerwartetes {}"),
    ("missing \")\"", "fehlende \")\""),
    ("incomplete expression", "unvollständiger Ausdruck"),
    ("unknown variable \"{}\"", "unbekannte Variable \"{}\""),
    ("division by zero", "Division durch null"),
    ("unknown operator \"{}\"", "unbekannter Operator \"{}\""),
    ("result is not a finite number", "Ergebnis ist keine endliche Zahl"),
    ("factorial needs a whole number from 0 to 170", "Fakultät braucht eine ganze Zahl von 0 bis 170"),
    ("{}() takes one value", "{}() erwartet einen Wert"),
    ("{}() needs at least one value", "{}() braucht mindestens einen Wert"),
    ("square root of a negative number", "Wurzel aus einer negativen Zahl"),
    ("round() takes a value and optional decimals", "round() erwartet einen Wert und optional die Nachkommastellen"),
    ("unknown function \"{}\"", "unbekannte Funktion \"{}\""),
    ("This is not a QuickNotes backup (.tar.gz)", "Das ist keine QuickNotes-Sicherung (.tar.gz)"),
    ("The backup is unexpectedly large", "Die Sicherung ist unerwartet groß"),
    ("The backup's session is damaged: {}", "Die Sitzung der Sicherung ist beschädigt: {}"),
    ("The backup was created by a newer QuickNotes version", "Die Sicherung stammt von einer neueren QuickNotes-Version"),
    ("GTK's \"Print to File\" printer is not available", "Der GTK-Drucker „In Datei drucken“ ist nicht verfügbar"),
    ("PDF export timed out", "Zeitüberschreitung beim PDF-Export"),
    ("Cannot start PlantUML ({}): {}", "PlantUML kann nicht gestartet werden ({}): {}"),
    ("PlantUML failed: {}", "PlantUML fehlgeschlagen: {}"),
    ("PlantUML produced no output", "PlantUML hat keine Ausgabe erzeugt"),
    ("The reStructuredText renderer failed on this document.", "Der reStructuredText-Renderer ist an diesem Dokument gescheitert."),
    ("QuickNotes Help", "QuickNotes-Hilfe"),
    ("Values: {}", "Werte: {}"),
    ("Word sums: {}", "Wortsummen: {}"),
    ("Sum: {}", "Summe: {}"),
    ("No numbers in the selection", "Keine Zahlen in der Auswahl"),
    ("\"{}\" is not a valid binary number", "\"{}\" ist keine gültige Binärzahl"),
    ("\"{}\" is not a valid hex number", "\"{}\" ist keine gültige Hexadezimalzahl"),
    ("\"{}\" is not a valid decimal number", "\"{}\" ist keine gültige Dezimalzahl"),
    ("{} is not a valid character code", "{} ist kein gültiger Zeichencode"),
    ("No letters in the selection", "Keine Buchstaben in der Auswahl"),
    ("Letter values: {}", "Buchstabenwerte: {}"),
    ("Word values: {}", "Buchstabenwortwerte: {}"),
    ("Sum of all letters: {}", "Summe aller Buchstaben (BWW): {}"),
    ("Letter checksums: {}", "Quersummen der Buchstabenwerte: {}"),
    ("Sum of letter checksums: {}", "Summe der Quersummen: {}"),
    ("Checksum of the sum: {}", "Quersumme der Summe: {}"),
    ("Single-digit checksum: {}", "Iterierte Quersumme: {}"),
    ("Switch on at least one output", "Mindestens eine Ausgabe einschalten"),
    ("Total: {}, different: {}", "Gesamt: {}, verschiedene: {}"),
    ("Missing: {}", "Fehlend: {}"),
    ("none", "keine"),
    ("Nothing to convert in the selection", "Nichts zum Umwandeln in der Auswahl"),
    ("Can't split \"{}\" into ASCII codes – please separate them", "\"{}\" lässt sich nicht in ASCII-Codes aufteilen – bitte mit Trennzeichen eingeben"),
    ("0 has no Roman numeral", "0 hat keine römische Zahl"),
    ("{} is too large for Roman numerals", "{} ist zu groß für römische Zahlen"),
    ("No Roman numerals in the selection", "Keine römischen Zahlen in der Auswahl"),
    ("No Morse code in the selection", "Kein Morsecode in der Auswahl"),
    ("Key-only (T9) codes are ambiguous and can't be decoded – use multi-tap or key + position", "Codes nur mit Tasten (T9) sind mehrdeutig und können nicht dekodiert werden – Mehrfachtippen oder Taste + Position verwenden"),
    ("\"{}\" is not a key sequence", "\"{}\" ist keine Tastenfolge"),
    ("\"{}\" is not a list of key + position pairs", "\"{}\" ist keine Folge von Taste-+-Position-Paaren"),
    ("ROT5 (digits)", "ROT5 (Ziffern)"),
    ("ROT18 (letters + digits)", "ROT18 (Buchstaben + Ziffern)"),
    ("Caesar Rotation", "Caesar-Verschiebung"),
    ("Shift", "Verschiebung"),
    ("Show all 25 rotations", "Alle 25 Verschiebungen anzeigen"),
    ("Letter Value", "Buchstabenwert (BWW)"),
    ("Direction", "Richtung"),
    ("Alphabet", "Alphabet"),
    ("Umlauts and ß", "Umlaute und ß"),
    ("Value of each letter", "Wert jedes Buchstabens"),
    ("Sum of each word", "Buchstabenwortwert jedes Wortes"),
    ("Sum of all letters", "Summe aller Buchstaben"),
    ("Checksum of each letter value", "Quersumme jedes Buchstabenwerts"),
    ("Checksum of the sum", "Quersumme der Summe"),
    ("Single-digit checksum of the sum", "Iterierte Quersumme der Summe"),
    ("Letter Frequency", "Buchstabenhäufigkeit"),
    ("Order", "Reihenfolge"),
    ("Count digits too", "Ziffern mitzählen"),
    ("List missing letters A–Z", "Fehlende Buchstaben A–Z auflisten"),
    ("Scrabble Value", "Scrabble-Wert"),
    ("Letter set", "Buchstabensatz"),
    ("Prime Alphabet", "Primzahl-Alphabet"),
    ("Sums", "Summen"),
    ("Hexadecimal", "Hexadezimal"),
    ("Conversion", "Umwandlung"),
    ("ASCII Codes", "ASCII-Codes"),
    ("Binary", "Binär"),
    ("Pad numbers to full bytes", "Zahlen auf volle Bytes auffüllen"),
    ("Roman Numerals", "Römische Zahlen"),
    ("Thousands from 4000 on", "Tausender ab 4000"),
    ("Morse Code", "Morsecode"),
    ("Dash symbol", "Zeichen für Strich"),
    ("Word separator", "Worttrenner"),
    ("Phone Keypad", "Handycode (Tastatur)"),
    ("Notation", "Schreibweise"),
    ("A=1 … Z=26", "A=1 … Z=26"),
    ("Z=1 … A=26 (reverse)", "Z=1 … A=26 (rückwärts)"),
    ("As AE, OE, UE, SS", "Als AE, OE, UE, SS"),
    ("Ä=27, Ö=28, Ü=29, ß=30", "Ä=27, Ö=28, Ü=29, ß=30"),
    ("Ignore", "Ignorieren"),
    ("Most frequent first", "Häufigste zuerst"),
    ("Alphabetical", "Alphabetisch"),
    ("German", "Deutsch"),
    ("English", "Englisch"),
    ("Hex → text", "Hex → Text"),
    ("Hex → decimal", "Hex → Dezimal"),
    ("Text → hex", "Text → Hex"),
    ("Decimal → hex", "Dezimal → Hex"),
    ("Codes → text", "Codes → Text"),
    ("Text → codes", "Text → Codes"),
    ("Binary → decimal", "Binär → Dezimal"),
    ("Binary → text", "Binär → Text"),
    ("Decimal → binary", "Dezimal → Binär"),
    ("Text → binary", "Text → Binär"),
    ("Detect", "Automatisch erkennen"),
    ("Roman → Arabic", "Römisch → Arabisch"),
    ("Arabic → Roman", "Arabisch → Römisch"),
    ("Overline (X̄ = 10000)", "Überstrich (X̄ = 10000)"),
    ("Brackets ((X) = 10000)", "Klammern ((X) = 10000)"),
    ("Text → Morse", "Text → Morse"),
    ("Morse → text", "Morse → Text"),
    ("Minus  -", "Minus  -"),
    ("Underscore  _", "Unterstrich  _"),
    ("Slash  /", "Schrägstrich  /"),
    ("Bar  |", "Senkrechter Strich  |"),
    ("Three spaces", "Drei Leerzeichen"),
    ("Text → keys", "Text → Tasten"),
    ("Keys → text", "Tasten → Text"),
    ("Multi-tap (GEO = 4 33 666)", "Mehrfachtippen (GEO = 4 33 666)"),
    ("Key + position (GEO = 41 32 63)", "Taste + Position (GEO = 41 32 63)"),
    ("Key only / T9 (GEO = 436)", "Nur Taste / T9 (GEO = 436)"),
    ("Letters → numbers", "Buchstaben → Zahlen"),
    ("Numbers → letters", "Zahlen → Buchstaben"),
    // Presentations
    ("From Beginning", "Von Anfang an"),
    ("From Current Slide", "Ab aktueller Folie"),
    ("In a Window", "In einem Fenster"),
    ("Start presentation (Ctrl+F5)", "Präsentation starten (Strg+F5)"),
    ("Presentation options", "Präsentationsoptionen"),
    ("This tab is no presentation", "Dieser Tab ist keine Präsentation"),
    ("The presentation has no slides yet", "Die Präsentation hat noch keine Folien"),
    ("Slide {} of {}", "Folie {} von {}"),
    ("New Presentation", "Neue Präsentation"),
    ("Start Presentation", "Präsentation starten"),
    ("Present from Current Slide", "Ab aktueller Folie präsentieren"),
    ("Present in a Window", "In einem Fenster präsentieren"),
    ("Slide", "Folie"),
    ("No slides yet. Separate slides with a line containing only ---", "Noch keine Folien. Folien werden durch eine Zeile mit nur --- getrennt"),
    ("Notes", "Notizen"),
    ("No notes for this slide", "Keine Notizen zu dieser Folie"),
    ("Next", "Als Nächstes"),
    ("Next step", "Nächster Schritt"),
    ("Next slide", "Nächste Folie"),
    ("End of presentation", "Ende der Präsentation"),
    ("Play the animations of this slide", "Animationen dieser Folie abspielen"),
    ("1 step", "1 Schritt"),
    ("{} steps", "{} Schritte"),
    ("Previous slide", "Vorherige Folie"),
    ("Elapsed time – click or T to pause, R to reset", "Verstrichene Zeit – Klick oder T pausiert, R setzt zurück"),
    ("remaining", "verbleibend"),
    ("over time", "überzogen"),
    ("Screen is black", "Bildschirm ist schwarz"),
    ("Screen is white", "Bildschirm ist weiß"),
    ("Black screen (B)", "Schwarzer Bildschirm (B)"),
    ("Swap screens (S)", "Bildschirme tauschen (S)"),
    ("End presentation (Esc)", "Präsentation beenden (Esc)"),
    ("Smaller notes (−)", "Notizen kleiner (−)"),
    ("Larger notes (+)", "Notizen größer (+)"),
    ("Keys (H)", "Tasten (H)"),
    ("Go to slide", "Gehe zu Folie"),
    ("→ ↓ Space PgDn Enter, click", "→ ↓ Leertaste Bild↓ Enter, Klick"),
    ("Next step or slide", "Nächster Schritt bzw. nächste Folie"),
    ("← ↑ PgUp Backspace, right click", "← ↑ Bild↑ Rücktaste, Rechtsklick"),
    ("Previous step or slide", "Vorheriger Schritt bzw. vorherige Folie"),
    ("Home / End", "Pos1 / Ende"),
    ("First / last slide", "Erste / letzte Folie"),
    ("Number + Enter", "Zahl + Enter"),
    ("B or .  /  W or ,", "B oder .  /  W oder ,"),
    ("Black / white screen", "Schwarzer / weißer Bildschirm"),
    ("Show/hide presenter view (one screen)", "Referentenansicht ein/aus (ein Bildschirm)"),
    ("Swap screens", "Bildschirme tauschen"),
    ("Toggle fullscreen", "Vollbild ein/aus"),
    ("Pause / reset the timer", "Zeit anhalten / zurücksetzen"),
    ("Notes font size", "Schriftgröße der Notizen"),
    ("End presentation", "Präsentation beenden"),
    ("Presentation", "Präsentation"),
    ("{} – Presenter View", "{} – Referentenansicht"),
    ("Presentation running", "Präsentation läuft"),
    ("Ctrl+Space lists the options at the cursor", "Strg+Leertaste listet die Möglichkeiten an der Schreibmarke"),
    ("Syntax Help", "Syntax-Hilfe"),
    ("Speed: 300 ms", "Tempo: 300 ms"),
    ("Speed: 600 ms", "Tempo: 600 ms"),
    ("Speed: 1 s", "Tempo: 1 s"),
    ("The whole block is one step", "Der ganze Block ist ein Schritt"),
    ("Each list item or paragraph is its own step", "Jeder Listenpunkt oder Absatz ist ein eigener Schritt"),
    ("Dims earlier steps of the container", "Blendet frühere Schritte des Containers ab"),
    ("Positions the block on the slide", "Positioniert den Block auf der Folie"),
    ("Places the block at the top", "Setzt den Block nach oben"),
    ("Places the block in the middle", "Setzt den Block in die Mitte"),
    ("Places the block at the bottom", "Setzt den Block nach unten"),
    ("Aligns left (with place: left edge)", "Linksbündig (mit place: linker Rand)"),
    ("Centers the text or the placed block", "Zentriert den Text oder den platzierten Block"),
    ("Aligns right (with place: right edge)", "Rechtsbündig (mit place: rechter Rand)"),
    ("Horizontal position in % of the slide", "Horizontale Position in % der Folie"),
    ("Vertical position in % of the slide", "Vertikale Position in % der Folie"),
    ("Width in % of the slide", "Breite in % der Folie"),
    ("Columns of equal width, ||| starts the next one", "Gleich breite Spalten, ||| beginnt die nächste"),
    ("Columns with this width ratio", "Spalten in diesem Breitenverhältnis"),
    ("Draws a card around the block", "Zeichnet eine Karte um den Block"),
    ("Smaller text", "Kleinerer Text"),
    ("Larger text", "Größerer Text"),
    ("Muted text color", "Gedämpfte Textfarbe"),
    ("Relative font size, e.g. size=1.3", "Relative Schriftgröße, z. B. size=1.3"),
    ("Waits before the animation, e.g. delay=200ms", "Wartet vor der Animation, z. B. delay=200ms"),
    ("Step animation", "Schritt-Animation"),
    ("Opens the file as presentation", "Öffnet die Datei als Präsentation"),
    ("Title of the presentation", "Titel der Präsentation"),
    ("Color theme", "Farbschema"),
    ("Slide format", "Folienformat"),
    ("Default slide transition", "Standard-Folienübergang"),
    ("Default animation of steps", "Standard-Animation der Schritte"),
    ("Slide numbers", "Foliennummern"),
    ("Text at the top of every slide", "Text oben auf jeder Folie"),
    ("Text at the bottom of every slide", "Text unten auf jeder Folie"),
    ("Talk length, e.g. 20m", "Vortragsdauer, z. B. 20m"),
    ("Extra CSS for the slides", "Zusätzliches CSS für die Folien"),
    ("Slide transition", "Folienübergang"),
    ("Animation of the steps on this slide", "Animation der Schritte auf dieser Folie"),
    ("Background color or gradient", "Hintergrundfarbe oder Verlauf"),
    ("Text color", "Textfarbe"),
    ("lead (title slide), invert (swapped colors)", "lead (Titelfolie), invert (vertauschte Farben)"),
    ("left, center, right", "left, center, right"),
    ("top, middle, bottom", "top, middle, bottom"),
    ("Color theme of this slide", "Farbschema dieser Folie"),
    ("Text at the top", "Text oben"),
    ("Text at the bottom", "Text unten"),
    ("Slide number on/off", "Foliennummer an/aus"),
    ("Horizontal alignment", "Horizontale Ausrichtung"),
    ("Vertical alignment", "Vertikale Ausrichtung"),
    ("Centered title slide", "Zentrierte Titelfolie"),
    ("Swapped colors", "Vertauschte Farben"),
    ("Duration", "Dauer"),
    ("Background image filling the slide", "Hintergrundbild, füllt die Folie"),
    ("Background: fits the whole image in", "Hintergrund: ganzes Bild einpassen"),
    ("Floats left; with bg: image on the left side", "Links umflossen; mit bg: Bild auf der linken Seite"),
    ("Floats right; with bg: image on the right side", "Rechts umflossen; mit bg: Bild auf der rechten Seite"),
    ("Centered image", "Zentriertes Bild"),
    ("Width, e.g. w=400 or w=50%", "Breite, z. B. w=400 oder w=50%"),
    ("Height, e.g. h=300", "Höhe, z. B. h=300"),
    ("With bg left/right: width of the image side", "Mit bg left/right: Breite der Bildseite"),
    ("Presentation syntax", "Präsentations-Syntax"),
    ("Container options (::: …)", "Container-Optionen (::: …)"),
    ("Presentation settings (front matter)", "Präsentations-Einstellungen (Front Matter)"),
    ("Slide settings (<!-- key: value; … -->)", "Folien-Einstellungen (<!-- key: value; … -->)"),
    ("Values for {}", "Werte für {}"),
    ("Image options (![…](file))", "Bild-Optionen (![…](datei))"),
    ("Starts a new slide", "Beginnt eine neue Folie"),
    ("Container: steps, animations, layout – closed by :::", "Container: Schritte, Animationen, Layout – endet mit :::"),
    ("Next column inside ::: columns", "Nächste Spalte in ::: columns"),
    ("Settings of this slide", "Einstellungen dieser Folie"),
    ("Image with size and alignment", "Bild mit Größe und Ausrichtung"),
    ("Background image", "Hintergrundbild"),
    ("Presenter notes follow", "Danach folgen die Sprechernotizen"),
    ("Distraction-free mode – Alt+Enter or Esc to leave", "Ablenkungsfreier Modus – mit Alt+Enter oder Esc verlassen"),
    ("Distraction-Free Mode", "Ablenkungsfreier Modus"),
    ("Code", "Code"),
    ("Documents (Markdown, reStructuredText, HTML, PlantUML)", "Dokumente (Markdown, reStructuredText, HTML, PlantUML)"),
    ("Presentations", "Präsentationen"),
    ("Calculations", "Berechnungen"),
    ("Images", "Bilder"),
    ("Preferences", "Einstellungen"),
    ("General", "Allgemein"),
    ("Dark", "Dunkel"),
    ("Light", "Hell"),
    ("Editor Font", "Editor-Schrift"),
    ("The size can also be changed with Ctrl+mouse wheel or Ctrl +/−/0.", "Die Größe lässt sich auch mit Strg+Mausrad oder Strg +/−/0 ändern."),
    ("Font", "Schriftart"),
    ("Size (pt)", "Größe (pt)"),
    ("Combines sequences like != or => in fonts such as Fira Code", "Verbindet Zeichenfolgen wie != oder => in Schriften wie Fira Code"),
    ("User Interface", "Benutzeroberfläche"),
    ("Applies after a restart", "Wirkt nach einem Neustart"),
    ("Tabs", "Tabs"),
    ("Layout", "Anordnung"),
    ("Tab Position", "Tab-Position"),
    ("Horizontal (top)", "Horizontal (oben)"),
    ("Vertical (side)", "Vertikal (seitlich)"),
    ("Tab Colours", "Tab-Farben"),
    ("Tabs are coloured by their type; plain notes keep the default look.", "Tabs werden nach ihrem Typ eingefärbt; einfache Notizen behalten das Standard-Aussehen."),
    ("Restore default colour", "Standardfarbe wiederherstellen"),
    ("Backup", "Sicherung"),
    ("All tabs, notes and settings in one .tar.gz file.", "Alle Tabs, Notizen und Einstellungen in einer .tar.gz-Datei."),
    ("Export all notes into a backup file", "Alle Notizen in eine Sicherungsdatei exportieren"),
    ("Add the tabs of a backup or replace the current ones", "Tabs einer Sicherung hinzufügen oder die aktuellen ersetzen"),
    ("Storage", "Speicherort"),
    ("Themes", "Designs"),
    ("Import/Export", "Import/Export"),
    ("Default", "Standard"),
    ("The theme is applied right away. Search themes by name with Ctrl+F.", "Das Design wird sofort angewendet. Mit Strg+F nach Namen suchen."),
    ("Default Dark", "Standard dunkel"),
    ("Default Light", "Standard hell"),
    ("Not available here", "Hier nicht verfügbar"),
    ("Invalid abbreviation", "Ungültige Abkürzung"),
    ("Expand Emmet Abbreviation", "Emmet-Abkürzung erweitern"),
    ("Wrap with Emmet Abbreviation…", "Mit Emmet-Abkürzung umschließen…"),
    ("Wrap with Emmet Abbreviation", "Mit Emmet-Abkürzung umschließen"),
    ("No Emmet abbreviation before the cursor", "Keine Emmet-Abkürzung vor dem Cursor"),
    ("Abbreviation, e.g. ul>li*", "Abkürzung, z. B. ul>li*"),
    ("Emmet: {}", "Emmet: {}"),
    ("Abbreviations like ul>li*3 or m10 in HTML, XML, XSLT, CSS, SCSS and Less files.", "Abkürzungen wie ul>li*3 oder m10 in HTML-, XML-, XSLT-, CSS-, SCSS- und Less-Dateien."),
    ("Expand with Tab", "Mit Tab erweitern"),
    ("Ctrl+E always expands; Ctrl+Shift+G wraps the selection", "Strg+E erweitert immer; Strg+Umschalt+G umschließt die Auswahl"),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// String literals passed to `t!(…)` in the sources (escapes resolved).
    fn used_keys() -> Vec<String> {
        let sources = [
            include_str!("window.rs"),
            include_str!("editor.rs"),
            include_str!("image_tab.rs"),
            include_str!("format.rs"),
            include_str!("calc.rs"),
            include_str!("backup.rs"),
            include_str!("export.rs"),
            include_str!("plantuml.rs"),
            include_str!("rst.rs"),
            include_str!("help.rs"),
            include_str!("geocaching.rs"),
            include_str!("slides.rs"),
            include_str!("slides_assist.rs"),
            include_str!("completion.rs"),
            include_str!("presenter.rs"),
            include_str!("prefs.rs"),
            include_str!("themes.rs"),
            include_str!("emmet.rs"),
        ];
        let mut keys = Vec::new();
        for src in sources {
            let parts: Vec<&str> = src.split("t!(\"").collect();
            for (prev, part) in parts.iter().zip(parts.iter().skip(1)) {
                // Skip `format!(` and other macros ending in "t".
                if prev.ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let mut key = String::new();
                let mut chars = part.chars();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => key.push('\n'),
                            Some('\n') => panic!("no line continuations in t!() strings"),
                            Some(c) => key.push(c),
                            None => {}
                        },
                        c => key.push(c),
                    }
                }
                keys.push(key);
            }
        }
        keys
    }

    #[test]
    fn every_used_string_is_translated() {
        let missing: Vec<String> = used_keys().into_iter().filter(|k| !german().contains_key(k.as_str())).collect();
        assert!(missing.is_empty(), "missing German translations: {missing:#?}");
    }

    #[test]
    fn geocaching_labels_are_translated() {
        for tool in crate::geocaching::TOOLS {
            let mut labels = vec![tool.label];
            for field in tool.fields {
                labels.extend(field.labels());
            }
            for label in labels.into_iter().filter(|l| !l.starts_with("ROT") || l.contains('(')) {
                assert!(german().contains_key(label), "missing German translation: {label}");
            }
        }
    }

    #[test]
    fn placeholders_match() {
        for (en, de) in DE {
            assert_eq!(en.matches("{}").count(), de.matches("{}").count(), "{en}");
        }
        let mut seen = std::collections::HashSet::new();
        for (en, _) in DE {
            assert!(seen.insert(en), "duplicate key {en}");
        }
    }

    #[test]
    fn fills_placeholders() {
        assert_eq!(fill("{} of {}", &[&1, &"x"]), "1 of x");
    }
}
