//! Importing data as Markdown tables: CSV/TSV and other delimited text
//! (also from the clipboard) and spreadsheets (XLSX, XLSM, XLS, ODS).

use std::path::Path;

/// Rows taken over at most; larger tables are cut.
pub const MAX_ROWS: usize = 5000;

/// A sheet of a spreadsheet, or the content of a text file.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

/// Field separators of delimited text.
pub const DELIMITERS: &[(u8, &str)] = &[(b',', "Comma"), (b';', "Semicolon"), (b'\t', "Tab"), (b'|', "Pipe")];

/// Whether a file is a spreadsheet (by its extension).
pub fn is_spreadsheet(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    matches!(ext.as_str(), "xlsx" | "xlsm" | "xlsb" | "xls" | "xla" | "ods")
}

/// Reads all sheets of a spreadsheet.
pub fn read_spreadsheet(path: &Path) -> Result<Vec<Sheet>, String> {
    use calamine::{open_workbook_auto, Reader};
    let mut book = open_workbook_auto(path).map_err(|e| e.to_string())?;
    let mut sheets = Vec::new();
    for name in book.sheet_names() {
        let range = book.worksheet_range(&name).map_err(|e| e.to_string())?;
        let rows = range.rows().map(|row| row.iter().map(cell_text).collect()).collect();
        sheets.push(Sheet { name, rows: tidy(rows) });
    }
    Ok(sheets)
}

fn cell_text(cell: &calamine::Data) -> String {
    use calamine::Data;
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_owned(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => number_text(*f),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => excel_date(d.as_f64(), d.is_datetime()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.replace('T', " "),
        Data::Error(e) => format!("#{e:?}"),
    }
}

/// A number without needless decimals (`3`, `2.5`, `0.1`).
fn number_text(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        // Shortest text that reads back as the same number, without the
        // noise of binary fractions (0.1 + 0.2).
        let rounded: f64 = format!("{f:.10}").parse().unwrap_or(f);
        rounded.to_string()
    }
}

/// ISO date (and time) of an Excel serial date (days since 1899-12-30).
fn excel_date(serial: f64, datetime: bool) -> String {
    if !datetime {
        // A duration: hours, minutes, seconds.
        let secs = (serial * 86400.0).round() as i64;
        return format!("{}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60);
    }
    let days = serial.floor() as i64;
    let secs = ((serial - days as f64) * 86400.0).round() as i64;
    let (y, m, d) = civil(days - 25569); // days since 1970-01-01
    let date = format!("{y:04}-{m:02}-{d:02}");
    if secs == 0 {
        date
    } else if secs >= 86400 {
        let (y, m, d) = civil(days - 25568);
        format!("{y:04}-{m:02}-{d:02}")
    } else {
        let s = if secs % 60 == 0 { String::new() } else { format!(":{:02}", secs % 60) };
        format!("{date} {:02}:{:02}{s}", secs / 3600, secs % 3600 / 60)
    }
}

/// Year, month and day of a day count since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
pub(crate) fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Text of a file: UTF-8 (with or without BOM), UTF-16 with BOM, or else
/// Windows-1252 (CSV files saved by Excel).
pub fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some((enc, _)) = encoding_rs::Encoding::for_bom(bytes) {
        return enc.decode_with_bom_removal(bytes).0.into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode_without_bom_handling(bytes).0.into_owned(),
    }
}

/// The separator of delimited text: the candidate that splits the first
/// lines into the same number of fields most consistently.
pub fn detect_delimiter(text: &str) -> u8 {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).take(20).collect();
    let mut best = (b',', 0usize);
    for &(d, _) in DELIMITERS {
        let counts: Vec<usize> = lines.iter().map(|l| fields_outside_quotes(l, d as char)).collect();
        let Some(&first) = counts.first() else { continue };
        if first == 0 {
            continue;
        }
        let consistent = counts.iter().filter(|&&c| c == first).count();
        // Lines with the same field count matter most, then more fields.
        let score = consistent * 1000 + first;
        if score > best.1 {
            best = (d, score);
        }
    }
    best.0
}

fn fields_outside_quotes(line: &str, sep: char) -> usize {
    let mut quoted = false;
    line.chars()
        .filter(|&c| {
            if c == '"' {
                quoted = !quoted;
            }
            c == sep && !quoted
        })
        .count()
}

/// Rows of delimited text (`None`: detect the separator).
pub fn read_delimited(text: &str, delimiter: Option<u8>) -> Result<Vec<Vec<String>>, String> {
    let delimiter = delimiter.unwrap_or_else(|| detect_delimiter(text));
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| e.to_string())?;
        rows.push(record.iter().map(str::to_owned).collect());
    }
    Ok(tidy(rows))
}

/// Drops empty rows and empty trailing columns.
fn tidy(rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = rows.into_iter().filter(|r| r.iter().any(|c| !c.trim().is_empty())).collect();
    let width = rows.iter().map(|r| r.iter().rposition(|c| !c.trim().is_empty()).map_or(0, |i| i + 1)).max().unwrap_or(0);
    for r in &mut rows {
        r.resize(width, String::new());
    }
    // Empty columns at the start (spreadsheets often have a margin).
    let lead = (0..width).take_while(|&c| rows.iter().all(|r| r[c].trim().is_empty())).count();
    if lead > 0 {
        for r in &mut rows {
            r.drain(..lead);
        }
    }
    rows
}

/// A Markdown table of `rows`. Without a header row the columns get
/// numbered names. Returns the table and whether rows were cut.
pub fn to_markdown(rows: &[Vec<String>], header: bool, indent: &str) -> (String, bool) {
    let width = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let (head, body): (Vec<String>, &[Vec<String>]) = if header && !rows.is_empty() {
        let head = (0..width)
            .map(|c| {
                let h = rows[0].get(c).map(|s| escape(s)).unwrap_or_default();
                if h.is_empty() {
                    t!("Column {}", c + 1)
                } else {
                    h
                }
            })
            .collect();
        (head, &rows[1..])
    } else {
        ((0..width).map(|c| t!("Column {}", c + 1)).collect(), rows)
    };
    let cut = body.len() > MAX_ROWS;
    let body = &body[..body.len().min(MAX_ROWS)];
    let line = |cells: Vec<String>| format!("{indent}| {} |", cells.join(" | "));
    let mut out = vec![line(head), line(vec!["---".to_owned(); width])];
    for r in body {
        out.push(line((0..width).map(|c| r.get(c).map(|s| escape(s)).unwrap_or_default()).collect()));
    }
    (crate::table::format_all(&out.join("\n")), cut)
}

/// A cell as Markdown table cell text.
fn escape(cell: &str) -> String {
    let lines: Vec<&str> = cell.trim().lines().map(str::trim).collect();
    lines.join("<br>").replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_delimiters() {
        assert_eq!(detect_delimiter("a,b,c\n1,2,3\n"), b',');
        assert_eq!(detect_delimiter("Name;Preis\nKäse;1,50\nBrot;2,30\n"), b';');
        assert_eq!(detect_delimiter("a\tb\n1\t2,5\n"), b'\t');
        assert_eq!(detect_delimiter("\"a,b\";c\n\"x,y\";z\n"), b';');
    }

    #[test]
    fn reads_csv() {
        let rows = read_delimited("Name;Preis\n\"Käse; alt\";1,50\n\n;\nBrot;2,30\n", None).unwrap();
        assert_eq!(rows, [["Name", "Preis"], ["Käse; alt", "1,50"], ["Brot", "2,30"]]);
        let rows = read_delimited("a,b,,\n1,2\n", None).unwrap();
        assert_eq!(rows, [["a", "b"], ["1", "2"]]);
    }

    #[test]
    fn decodes_files() {
        assert_eq!(decode(b"\xEF\xBB\xBFa;b"), "a;b");
        assert_eq!(decode(b"K\xE4se"), "Käse");
        assert_eq!(decode("Käse".as_bytes()), "Käse");
        assert_eq!(decode(b"\xFF\xFEa\x00b\x00"), "ab");
    }

    #[test]
    fn markdown_tables() {
        let rows = vec![vec!["Name".to_owned(), "Note".to_owned()], vec!["a|b".to_owned(), "x\ny".to_owned()]];
        let (md, cut) = to_markdown(&rows, true, "");
        assert!(!cut);
        assert_eq!(md, "| Name | Note   |\n| ---- | ------ |\n| a\\|b | x<br>y |");
        let (md, _) = to_markdown(&rows[1..], false, "");
        assert!(md.starts_with("| Column 1 | Column 2 |"), "{md}");
    }

    #[test]
    fn numbers_and_dates() {
        assert_eq!(number_text(3.0), "3");
        assert_eq!(number_text(2.5), "2.5");
        assert_eq!(number_text(0.1 + 0.2), "0.3");
        assert_eq!(excel_date(45658.0, true), "2025-01-01");
        assert_eq!(excel_date(45658.5, true), "2025-01-01 12:00");
        assert_eq!(excel_date(1.5, false), "36:00:00");
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_723), (2024, 1, 1));
    }

    #[test]
    fn tidies_rows() {
        let rows = vec![vec!["".into(), "a".into(), "".into()], vec!["".into(), "".into(), "".into()], vec!["".into(), "1".into()]];
        assert_eq!(tidy(rows), [["a"], ["1"]]);
    }

    #[test]
    fn spreadsheet_extensions() {
        assert!(is_spreadsheet(Path::new("x.XLSX")));
        assert!(is_spreadsheet(Path::new("x.ods")));
        assert!(!is_spreadsheet(Path::new("x.csv")));
    }
}
