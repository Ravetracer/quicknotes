//! Charts in Markdown documents and presentations, drawn with Apache
//! ECharts from the data of Markdown tables.
//!
//! A ```` ```chart ```` block holds the chart's settings as YAML; it takes
//! its data from a table of the document (`table: above`, `below`, a number
//! or a heading). Defaults for all charts of a document go into the front
//! matter under `charts:`.
//!
//! [`expand`] resolves the blocks into ```` ```qn-chart ```` blocks holding
//! the ECharts option as JSON (keeping the line count, as slides and task
//! checkboxes refer to source lines); the page script draws them (see
//! [`crate::notation`]).

use crate::chart_data::{self, Column, Kind, Outline, Table, Value};
use serde_json::{json, Map, Value as Json};
use serde_yaml::{Mapping, Value as Yaml};
use std::borrow::Cow;

/// Info string of chart blocks in Markdown.
pub const LANG: &str = "chart";
/// Info string of resolved chart blocks (HTML class `language-qn-chart`).
pub const RESOLVED: &str = "qn-chart";

/// Chart types and their aliases.
pub const TYPES: &[(&str, &[&str])] = &[
    ("line", &[]),
    ("area", &[]),
    ("bar", &["column"]),
    ("pie", &[]),
    ("donut", &["doughnut", "ring"]),
    ("rose", &["nightingale"]),
    ("scatter", &[]),
    ("bubble", &[]),
    ("radar", &["spider"]),
    ("funnel", &[]),
    ("gauge", &[]),
    ("boxplot", &["box"]),
    ("candlestick", &["ohlc", "kline"]),
    ("heatmap", &[]),
    ("calendar", &[]),
    ("sankey", &[]),
    ("graph", &["network"]),
    ("treemap", &[]),
    ("sunburst", &[]),
    ("tree", &[]),
    ("parallel", &[]),
    ("themeriver", &["river", "streamgraph"]),
];

/// Settings of a chart block.
pub const KEYS: &[&str] = &[
    "type", "table", "title", "subtitle", "x", "y", "y2", "group", "size", "label", "value", "name", "parent", "path",
    "source", "target", "open", "high", "low", "close", "stack", "horizontal", "smooth", "step", "area", "labels",
    "legend", "zoom", "sort", "directed", "layout", "min", "max", "log", "unit", "decimal", "height", "theme",
    "palette", "colors", "animation", "echarts",
];

/// Named colour palettes (from the ECharts themes, and the app's colours).
pub const PALETTES: &[(&str, &[&str])] = &[
    ("default", &["#5470c6", "#91cc75", "#fac858", "#ee6666", "#73c0de", "#3ba272", "#fc8452", "#9a60b4", "#ea7ccc"]),
    ("gnome", &["#3584e4", "#e66100", "#26a269", "#e01b24", "#9141ac", "#2190a4", "#c88800", "#c061cb", "#986a44", "#77767b"]),
    ("macarons", &["#2ec7c9", "#b6a2de", "#5ab1ef", "#ffb980", "#d87a80", "#8d98b3", "#e5cf0d", "#97b552", "#95706d", "#dc69aa"]),
    ("vintage", &["#d87c7c", "#919e8b", "#d7ab82", "#6e7074", "#61a0a8", "#efa18d", "#787464", "#cc7e63", "#724e58", "#4b565b"]),
    ("roma", &["#e01f54", "#001852", "#f5e8c8", "#b8d2c7", "#c6b38e", "#a4d8c2", "#f3d999", "#d3758f", "#dcc392", "#2e4783"]),
    ("shine", &["#c12e34", "#e6b600", "#0098d9", "#2b821d", "#005eaa", "#339ca8", "#cda819", "#32a487"]),
    ("infographic", &["#c1232b", "#27727b", "#fcce10", "#e87c25", "#b5c334", "#fe8463", "#9bca63", "#fad860", "#f3a43b", "#60c0dd"]),
    ("royal", &["#3f7ea6", "#993366", "#408000", "#8c6f56", "#a65149", "#731f17", "#adc2eb", "#d9c3b0"]),
    ("sakura", &["#e52c3c", "#f7b1ab", "#fa506c", "#f59288", "#f8c4d8", "#e54f5c", "#f06d5c", "#e54f80", "#f29c9f", "#eeb5b7"]),
    ("caravan", &["#fad089", "#ff9c5b", "#f5634a", "#ed303c", "#3b8183", "#f7826e", "#faac9e", "#fcd5cf"]),
    ("azul", &["#f2385a", "#f5a503", "#4ad9d9", "#f7879c", "#c1d7a8", "#4dffd2", "#fccfd7", "#d5f6f6"]),
    ("helianthus", &["#44b7d3", "#e42b6d", "#f4e24e", "#fe9616", "#8aed35", "#ff69b4", "#ba55d3", "#cd5c5c", "#ffa500", "#40e0d0"]),
    ("blue", &["#1790cf", "#1bb2d8", "#99d2dd", "#88b0bb", "#1c7099", "#038cc4", "#75abd0", "#afd6dd"]),
    ("green", &["#408829", "#68a54a", "#a9cba2", "#86b379", "#397b29", "#8abb6f", "#759c6a", "#bfd3b7"]),
    ("red", &["#d8361b", "#f16b4c", "#f7b4a9", "#d26666", "#99311c", "#c42703", "#d07e75"]),
    ("gray", &["#757575", "#c7c7c7", "#dadada", "#8b8b8b", "#b5b5b5", "#e9e9e9"]),
];

const DEFAULT_HEIGHT: u32 = 360;

/// Chart types of the formatting toolbar: the common ones, then the rest.
pub const MENU_COMMON: &[(&str, &str)] =
    &[("line", "Line Chart"), ("area", "Area Chart"), ("bar", "Bar Chart"), ("pie", "Pie Chart"), ("donut", "Donut Chart"), ("scatter", "Scatter Plot")];
pub const MENU_MORE: &[(&str, &str)] = &[
    ("bubble", "Bubble Chart"),
    ("rose", "Rose Chart"),
    ("radar", "Radar Chart"),
    ("funnel", "Funnel"),
    ("gauge", "Gauge"),
    ("boxplot", "Box Plot"),
    ("candlestick", "Candlestick (OHLC)"),
    ("heatmap", "Heat Map"),
    ("calendar", "Calendar Heat Map"),
    ("sankey", "Sankey Diagram"),
    ("graph", "Network Graph"),
    ("treemap", "Treemap"),
    ("sunburst", "Sunburst"),
    ("tree", "Tree"),
    ("parallel", "Parallel Coordinates"),
    ("themeriver", "Theme River"),
];

/// Sample table and extra settings of a new chart.
fn sample(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "pie" | "donut" | "rose" => (
            "| Category | Amount |\n| -------- | ------ |\n| Rent     | 900    |\n| Food     | 400    |\n| Travel   | 250    |\n| Other    | 150    |",
            "",
        ),
        "funnel" => ("| Stage   | Count |\n| ------- | ----- |\n| Visits  | 1000  |\n| Signups | 420   |\n| Trials  | 160   |\n| Orders  | 64    |", ""),
        "scatter" => (
            "| Height (cm) | Weight (kg) |\n| ----------- | ----------- |\n| 160         | 55          |\n| 168         | 63          |\n| 175         | 72          |\n| 182         | 80          |\n| 190         | 86          |",
            "",
        ),
        "bubble" => (
            "| Country | GDP per head | Life expectancy | Population (M) |\n| ------- | ------------ | --------------- | -------------- |\n| A       | 12000        | 71              | 120            |\n| B       | 34000        | 79              | 45             |\n| C       | 52000        | 82              | 83             |\n| D       | 8000         | 66              | 210            |",
            "",
        ),
        "radar" => ("| Skill   | Anna | Ben |\n| ------- | ---- | --- |\n| Rust    | 8    | 5   |\n| SQL     | 6    | 9   |\n| Design  | 7    | 4   |\n| Testing | 9    | 7   |\n| Writing | 5    | 8   |", ""),
        "gauge" => ("| Server | Load (%) |\n| ------ | -------- |\n| Web    | 72       |\n| DB     | 41       |", ""),
        "boxplot" => ("| Run A | Run B | Run C |\n| ----- | ----- | ----- |\n| 12    | 15    | 9     |\n| 14    | 17    | 11    |\n| 13    | 22    | 10    |\n| 15    | 16    | 14    |\n| 30    | 18    | 12    |", ""),
        "candlestick" => (
            "| Date       | Open | High | Low | Close |\n| ---------- | ---- | ---- | --- | ----- |\n| 2026-03-02 | 100  | 108  | 98  | 106   |\n| 2026-03-03 | 106  | 110  | 101 | 103   |\n| 2026-03-04 | 103  | 104  | 95  | 97    |\n| 2026-03-05 | 97   | 105  | 96  | 104   |\n| 2026-03-06 | 104  | 112  | 103 | 111   |",
            "",
        ),
        "heatmap" => ("| Day | 8h | 12h | 16h | 20h |\n| --- | -- | --- | --- | --- |\n| Mon | 2  | 8   | 5   | 1   |\n| Tue | 3  | 9   | 6   | 2   |\n| Wed | 1  | 7   | 8   | 4   |\n| Thu | 4  | 6   | 9   | 3   |\n| Fri | 5  | 10  | 4   | 7   |", ""),
        "calendar" => ("| Date       | Steps |\n| ---------- | ----- |\n| 2026-01-05 | 8200  |\n| 2026-01-06 | 4100  |\n| 2026-01-07 | 12050 |\n| 2026-02-14 | 9800  |\n| 2026-03-21 | 15000 |", ""),
        "sankey" => (
            "| From   | To      | Amount |\n| ------ | ------- | ------ |\n| Salary | Budget  | 3000   |\n| Bonus  | Budget  | 500    |\n| Budget | Rent    | 1100   |\n| Budget | Food    | 600    |\n| Budget | Savings | 1800   |",
            "",
        ),
        "graph" => ("| From  | To    |\n| ----- | ----- |\n| Alice | Bob   |\n| Alice | Carol |\n| Bob   | Dave  |\n| Carol | Dave  |\n| Dave  | Eve   |", "directed: true\n"),
        "treemap" | "sunburst" => (
            "| Region  | Country | Sales |\n| ------- | ------- | ----- |\n| Europe  | Germany | 420   |\n| Europe  | France  | 310   |\n| Europe  | Spain   | 150   |\n| America | USA     | 520   |\n| America | Canada  | 120   |\n| Asia    | Japan   | 260   |",
            "",
        ),
        "tree" => ("| Name     | Parent   |\n| -------- | -------- |\n| CEO      |          |\n| CTO      | CEO      |\n| CFO      | CEO      |\n| Dev Team | CTO      |\n| Ops Team | CTO      |\n| Finance  | CFO      |", "parent: Parent\n"),
        "parallel" => (
            "| Car | Power (hp) | Price (€) | Consumption (l) | Class   |\n| --- | ---------- | --------- | --------------- | ------- |\n| A   | 90         | 18000     | 5.1             | small   |\n| B   | 150        | 32000     | 6.4             | compact |\n| C   | 250        | 55000     | 8.9             | luxury  |\n| D   | 120        | 26000     | 5.8             | compact |",
            "group: Car\n",
        ),
        "themeriver" => (
            "| Date       | Rock | Pop | Jazz |\n| ---------- | ---- | --- | ---- |\n| 2026-01-01 | 10   | 20  | 5    |\n| 2026-02-01 | 14   | 18  | 7    |\n| 2026-03-01 | 12   | 25  | 6    |\n| 2026-04-01 | 18   | 22  | 9    |",
            "",
        ),
        _ => ("| Month | Revenue | Costs |\n| ----- | ------- | ----- |\n| Jan   | 1200    | 800   |\n| Feb   | 1500    | 900   |\n| Mar   | 1100    | 950   |\n| Apr   | 1800    | 1000  |", ""),
    }
}

/// A new chart block of type `kind` (with a sample table before it when
/// `with_table`) and the line of its title setting, for the cursor.
pub fn template(kind: &str, with_table: bool) -> (String, usize) {
    let (table, extra) = sample(kind);
    let title = MENU_COMMON.iter().chain(MENU_MORE).find(|(k, _)| *k == kind).map_or("Chart", |(_, t)| crate::i18n::tr(t));
    let block = format!("```{LANG}\ntype: {kind}\ntitle: {title}\n{extra}```");
    if with_table {
        (format!("{table}\n\n{block}"), table.lines().count() + 3)
    } else {
        (block, 2)
    }
}

// ----- expansion ---------------------------------------------------------------

/// A chart block in the source.
struct Block {
    /// Line of the opening fence, its indentation and fence characters.
    open: usize,
    indent: String,
    fence: String,
    /// Lines between the fences.
    body: std::ops::Range<usize>,
    yaml: String,
}

/// Replaces the chart blocks of a Markdown document by resolved ones.
pub fn expand(markdown: &str) -> Cow<'_, str> {
    if !markdown.contains(LANG) {
        return Cow::Borrowed(markdown);
    }
    let blocks = blocks(markdown);
    if blocks.is_empty() {
        return Cow::Borrowed(markdown);
    }
    let mut lines: Vec<String> = markdown.split('\n').map(str::to_owned).collect();
    let defaults = {
        let refs: Vec<&str> = markdown.split('\n').take(200).collect();
        defaults(&refs)
    };
    let outline = chart_data::outline(markdown);
    for b in blocks.iter().rev() {
        let resolved = resolve(&b.yaml, defaults.as_ref(), &outline, b.open, b.body.end);
        lines[b.open] = format!("{}{}{RESOLVED}", b.indent, b.fence);
        let json = format!("{}{resolved}", b.indent);
        if b.body.is_empty() {
            lines.insert(b.body.start, json);
        } else {
            lines[b.body.start] = json;
            for line in &mut lines[b.body.start + 1..b.body.end] {
                line.clear();
            }
        }
    }
    Cow::Owned(lines.join("\n"))
}

/// Chart blocks that start a line (not in quotes) of the document.
fn blocks(markdown: &str) -> Vec<Block> {
    use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
    let starts: Vec<usize> = std::iter::once(0).chain(markdown.match_indices('\n').map(|(i, _)| i + 1)).collect();
    let line_of = |offset: usize| starts.partition_point(|&s| s <= offset).saturating_sub(1);
    let line_text = |line: usize| {
        let end = starts.get(line + 1).map_or(markdown.len(), |e| e - 1);
        markdown[starts[line]..end].trim_end_matches('\r')
    };
    let mut out = Vec::new();
    let mut current: Option<(std::ops::Range<usize>, String)> = None;
    let opts = Options::ENABLE_TABLES | Options::ENABLE_MATH;
    for (event, range) in Parser::new_ext(markdown, opts).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) if info.split_whitespace().next() == Some(LANG) => {
                current = Some((range, String::new()));
            }
            Event::Text(t) => {
                if let Some((_, text)) = current.as_mut() {
                    text.push_str(&t);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let Some((range, yaml)) = current.take() else { continue };
                let open = line_of(range.start);
                let text = line_text(open);
                let trimmed = text.trim_start();
                let indent = &text[..text.len() - trimmed.len()];
                let marker = trimmed.chars().next().unwrap_or('`');
                let fence: String = trimmed.chars().take_while(|&c| c == marker).collect();
                if !indent.chars().all(|c| c == ' ') || indent.len() > 3 || fence.len() < 3 {
                    continue;
                }
                let last = line_of(range.end.saturating_sub(1).max(range.start));
                let closing = last > open && {
                    let t = line_text(last).trim();
                    t.len() >= fence.len() && t.chars().all(|c| c == marker)
                };
                let end = if closing { last } else { last + 1 };
                out.push(Block { open, indent: indent.to_owned(), fence, body: open + 1..end.max(open + 1), yaml });
            }
            _ => {}
        }
    }
    out
}

/// The `charts:` defaults in the front matter.
fn defaults(lines: &[&str]) -> Option<Mapping> {
    let (front, _) = crate::slides::front_matter(lines);
    match front?.get("charts")? {
        Yaml::Mapping(m) => Some(m.clone()),
        _ => None,
    }
}

/// JSON for a resolved chart block: `{"option": …, "height": …, "theme": …}`
/// or `{"error": …}`.
fn resolve(yaml: &str, defaults: Option<&Mapping>, outline: &Outline, open: usize, close: usize) -> String {
    let result = Spec::parse(yaml, defaults).and_then(|spec| build(&spec, outline, open, close));
    let json = match result {
        Ok(json) => json,
        Err(e) => json!({ "error": e }),
    };
    serde_json::to_string(&json).unwrap_or_default()
}

// ----- settings ----------------------------------------------------------------

struct Spec(Mapping);

fn normalize_key(key: &Yaml) -> Option<String> {
    let key = match key {
        Yaml::String(s) => s.clone(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Bool(b) => b.to_string(),
        _ => return None,
    };
    Some(key.trim().to_lowercase().replace('-', "_"))
}

impl Spec {
    fn parse(yaml: &str, defaults: Option<&Mapping>) -> Result<Spec, String> {
        let own = if yaml.trim().is_empty() {
            Mapping::new()
        } else {
            match serde_yaml::from_str::<Yaml>(yaml) {
                Ok(Yaml::Mapping(m)) => m,
                Ok(Yaml::Null) => Mapping::new(),
                Ok(_) => return Err(t!("Chart settings must be `key: value` lines").into()),
                Err(e) => return Err(t!("Invalid chart settings: {}", e)),
            }
        };
        let mut map = Mapping::new();
        for (layer, check) in [(defaults, false), (Some(&own), true)] {
            for (k, v) in layer.into_iter().flatten() {
                let Some(key) = normalize_key(k) else { continue };
                if !KEYS.contains(&key.as_str()) {
                    if check {
                        return Err(t!("Unknown chart setting \"{}\"", key));
                    }
                    continue;
                }
                let key = Yaml::String(key);
                let merged = match (map.get(&key), v) {
                    (Some(Yaml::Mapping(old)), Yaml::Mapping(new)) => {
                        let mut m = old.clone();
                        m.extend(new.iter().map(|(a, b)| (a.clone(), b.clone())));
                        Yaml::Mapping(m)
                    }
                    _ => v.clone(),
                };
                map.insert(key, merged);
            }
        }
        Ok(Spec(map))
    }

    fn get(&self, key: &str) -> Option<&Yaml> {
        self.0.get(key).filter(|v| !v.is_null())
    }

    fn text(&self, key: &str) -> Option<String> {
        match self.get(key)? {
            Yaml::String(s) => Some(s.trim().to_owned()),
            Yaml::Number(n) => Some(n.to_string()),
            Yaml::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    fn flag(&self, key: &str) -> Option<bool> {
        match self.get(key)? {
            Yaml::Bool(b) => Some(*b),
            Yaml::String(s) => match s.trim().to_lowercase().as_str() {
                "true" | "yes" | "on" | "ja" => Some(true),
                "false" | "no" | "off" | "nein" | "none" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    fn number(&self, key: &str) -> Option<f64> {
        match self.get(key)? {
            Yaml::Number(n) => n.as_f64(),
            Yaml::String(s) => chart_data::number(s, '.').map(|n| n.value),
            _ => None,
        }
    }

    /// One name or a list of names (`y: Revenue`, `y: [Revenue, Costs]`).
    fn names(&self, key: &str) -> Option<Vec<String>> {
        match self.get(key)? {
            Yaml::Sequence(seq) => Some(seq.iter().filter_map(yaml_text).collect()),
            v => yaml_text(v).map(|s| vec![s]),
        }
    }
}

fn yaml_text(v: &Yaml) -> Option<String> {
    match v {
        Yaml::String(s) => Some(s.trim().to_owned()),
        Yaml::Number(n) => Some(n.to_string()),
        Yaml::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn yaml_to_json(v: &Yaml) -> Json {
    serde_json::to_value(v).unwrap_or(Json::Null)
}

fn chart_type(spec: &Spec) -> Result<&'static str, String> {
    let name = spec.text("type").unwrap_or_else(|| "bar".into()).to_lowercase().replace(['-', '_', ' '], "");
    TYPES
        .iter()
        .find(|(t, aliases)| *t == name || aliases.contains(&name.as_str()))
        .map(|(t, _)| *t)
        .ok_or_else(|| t!("Unknown chart type \"{}\"", name))
}

// ----- data --------------------------------------------------------------------

/// The table a chart uses (`table:` setting).
fn find_table<'a>(spec: &Spec, outline: &'a Outline, open: usize, close: usize) -> Result<&'a Table, String> {
    let above = || outline.tables.iter().rev().find(|t| t.last < open);
    let below = || outline.tables.iter().find(|t| t.first > close);
    let Some(wanted) = spec.text("table") else {
        return above().or_else(below).ok_or_else(|| t!("No table found for the chart").into());
    };
    match wanted.to_lowercase().as_str() {
        "above" | "before" | "previous" | "oben" => return above().ok_or_else(|| t!("No table above the chart").into()),
        "below" | "after" | "next" | "unten" => return below().ok_or_else(|| t!("No table below the chart").into()),
        _ => {}
    }
    if let Ok(n) = wanted.parse::<usize>() {
        return n.checked_sub(1).and_then(|i| outline.tables.get(i)).ok_or_else(|| t!("There is no table {}", n));
    }
    let name = wanted.trim_start_matches('#').trim().to_lowercase();
    let heading = outline
        .headings
        .iter()
        .position(|h| h.text.to_lowercase() == name)
        .ok_or_else(|| t!("No heading \"{}\" found", wanted))?;
    let h = &outline.headings[heading];
    let end = outline.headings[heading + 1..].iter().find(|o| o.level <= h.level).map_or(usize::MAX, |o| o.line);
    outline
        .tables
        .iter()
        .find(|t| t.first > h.line && t.first < end)
        .ok_or_else(|| t!("No table under the heading \"{}\"", wanted))
}

struct Data {
    cols: Vec<Column>,
    rows: usize,
}

impl Data {
    fn new(table: &Table, decimal: Option<char>) -> Data {
        let fallback = match crate::i18n::lang() {
            crate::i18n::Lang::De => ',',
            crate::i18n::Lang::En => '.',
        };
        let width = table.header.len().max(table.rows.iter().map(Vec::len).max().unwrap_or(0));
        let cols = (0..width)
            .map(|c| {
                let raw = table.rows.iter().map(|r| r.get(c).cloned().unwrap_or_default()).collect();
                let name = table.header.get(c).cloned().unwrap_or_default();
                Column::parse(&name, raw, decimal, fallback)
            })
            .collect();
        Data { cols, rows: table.rows.len() }
    }

    /// Index of a column given by name or by number (1-based).
    fn find(&self, name: &str) -> Result<usize, String> {
        let wanted = name.trim().to_lowercase();
        if let Some(i) = self.cols.iter().position(|c| c.name.trim().to_lowercase() == wanted) {
            return Ok(i);
        }
        if let Ok(n) = wanted.parse::<usize>() {
            if (1..=self.cols.len()).contains(&n) {
                return Ok(n - 1);
            }
        }
        let names: Vec<&str> = self.cols.iter().map(|c| c.name.as_str()).collect();
        Err(t!("Column \"{}\" not found (columns: {})", name, names.join(", ")))
    }

    fn first_of(&self, kind: Kind, except: &[usize]) -> Option<usize> {
        (0..self.cols.len()).find(|i| !except.contains(i) && self.cols[*i].kind == kind)
    }
}

/// A chart under construction.
struct Chart<'a> {
    spec: &'a Spec,
    data: &'a Data,
    kind: &'static str,
    option: Map<String, Json>,
    /// Space taken at the top by title (and legend).
    top: u32,
    bottom: u32,
}

impl<'a> Chart<'a> {
    fn col(&self, i: usize) -> &Column {
        &self.data.cols[i]
    }

    /// A column given by setting `key`, if set.
    fn opt_col(&self, key: &str) -> Result<Option<usize>, String> {
        self.spec.names(key).and_then(|n| n.into_iter().next()).map(|n| self.data.find(&n)).transpose()
    }

    fn x(&self) -> Result<usize, String> {
        Ok(self.opt_col("x")?.unwrap_or(0))
    }

    /// Columns given by setting `key` or else all number columns not in `except`.
    fn numbers(&self, key: &str, except: &[usize]) -> Result<Vec<usize>, String> {
        if let Some(names) = self.spec.names(key) {
            let cols = names.iter().map(|n| self.data.find(n)).collect::<Result<Vec<_>, _>>()?;
            for &c in &cols {
                if self.col(c).kind == Kind::Text {
                    return Err(t!("Column \"{}\" does not hold numbers", self.col(c).name));
                }
            }
            return Ok(cols);
        }
        let cols: Vec<usize> =
            (0..self.data.cols.len()).filter(|i| !except.contains(i) && self.col(*i).kind == Kind::Number).collect();
        if cols.is_empty() {
            return Err(t!("The table has no column with numbers").into());
        }
        Ok(cols)
    }

    /// The value column: setting `value`, else `y`, else the first number column.
    fn value(&self, except: &[usize]) -> Result<usize, String> {
        if let Some(c) = self.opt_col("value")? {
            return Ok(c);
        }
        Ok(self.numbers("y", except)?[0])
    }

    fn num(&self, col: usize, row: usize) -> Json {
        self.col(col).num(row).map_or(Json::Null, |v| json!(v))
    }

    fn values(&self, col: usize) -> Vec<Json> {
        (0..self.data.rows).map(|r| self.num(col, r)).collect()
    }

    fn labels(&self, col: usize) -> Vec<String> {
        (0..self.data.rows).map(|r| self.col(col).label(r)).collect()
    }

    /// Distinct labels of a column, in order of appearance.
    fn distinct(&self, col: usize) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for l in self.labels(col) {
            if !out.contains(&l) {
                out.push(l);
            }
        }
        out
    }

    /// The cell as JSON value for a value, time or category axis.
    fn cell(&self, col: usize, row: usize) -> Json {
        match self.col(col).values.get(row) {
            Some(Value::Num(v)) => json!(v),
            Some(Value::Date(d)) => json!(d),
            Some(Value::Text(t)) => json!(t),
            _ => Json::Null,
        }
    }

    fn set(&mut self, key: &str, value: Json) {
        self.option.insert(key.to_owned(), value);
    }

    /// The legend, shown if set or by default. Reserves its space.
    fn legend(&mut self, default: bool) {
        let setting = self.spec.text("legend").map(|s| s.to_lowercase());
        let pos = match setting.as_deref() {
            None => if default { "bottom" } else { "none" },
            Some("true" | "yes" | "on") => "bottom",
            Some("false" | "no" | "off" | "none") => "none",
            Some(p) => p,
        };
        let legend = match pos {
            "none" => return,
            "top" => {
                let l = json!({ "type": "scroll", "top": self.top });
                self.top += 30;
                l
            }
            "left" => json!({ "type": "scroll", "orient": "vertical", "left": 0, "top": "middle" }),
            "right" => json!({ "type": "scroll", "orient": "vertical", "right": 0, "top": "middle" }),
            _ => {
                self.bottom += 30;
                json!({ "type": "scroll", "bottom": 0 })
            }
        };
        self.set("legend", legend);
    }

    fn labels_on(&self, default: bool) -> bool {
        self.spec.flag("labels").unwrap_or(default)
    }

    /// Axis label template for a unit (`{value} €`): the `unit` setting or
    /// the unit all columns share.
    fn unit_format(&self, cols: &[usize]) -> Option<String> {
        self.spec.text("unit").or_else(|| self.column_unit(cols)).map(|u| unit_template(&u))
    }

    fn column_unit(&self, cols: &[usize]) -> Option<String> {
        let first = self.col(*cols.first()?).unit.clone()?;
        cols.iter().all(|&c| self.col(c).unit.as_deref() == Some(first.as_str())).then_some(first)
    }

    fn value_axis(&self, cols: &[usize]) -> Json {
        let log = self.spec.flag("log").unwrap_or(false);
        json!({
            "type": if log { "log" } else { "value" },
            "min": self.spec.number("min"),
            "max": self.spec.number("max"),
            "axisLabel": { "formatter": self.unit_format(cols) },
        })
    }

    fn grid(&self) -> Json {
        json!({ "left": 16, "right": 24, "top": self.top + 16, "bottom": self.bottom + 12 })
    }

    fn zoom(&mut self, horizontal: bool, default: bool) {
        if !self.spec.flag("zoom").unwrap_or(default) {
            return;
        }
        let axis = if horizontal { "yAxisIndex" } else { "xAxisIndex" };
        let slider = if horizontal {
            json!({ "type": "slider", axis: 0, "right": 0 })
        } else {
            json!({ "type": "slider", axis: 0, "bottom": self.bottom })
        };
        if !horizontal {
            self.bottom += 40;
        }
        self.set("dataZoom", json!([{ "type": "inside", axis: 0 }, slider]));
    }

    /// Row order for `sort: asc|desc` by column `col`.
    fn order(&self, col: usize) -> Vec<usize> {
        let mut rows: Vec<usize> = (0..self.data.rows).collect();
        let key = |r: &usize| self.col(col).num(*r).unwrap_or(f64::NEG_INFINITY);
        match self.spec.text("sort").map(|s| s.to_lowercase()).as_deref() {
            Some("asc" | "ascending") => rows.sort_by(|a, b| key(a).total_cmp(&key(b))),
            Some("desc" | "descending") => rows.sort_by(|a, b| key(b).total_cmp(&key(a))),
            _ => {}
        }
        rows
    }
}

// ----- building ----------------------------------------------------------------

fn build(spec: &Spec, outline: &Outline, open: usize, close: usize) -> Result<Json, String> {
    let kind = chart_type(spec)?;
    let decimal = spec.text("decimal").and_then(|d| match d.as_str() {
        "," | "comma" | "komma" => Some(','),
        "." | "point" | "dot" | "punkt" => Some('.'),
        _ => None,
    });
    // A gauge can show a fixed value without a table.
    let literal_gauge = kind == "gauge" && matches!(spec.get("value"), Some(Yaml::Number(_)));
    let empty = Table { first: 0, last: 0, header: Vec::new(), rows: Vec::new() };
    let table = if literal_gauge { &empty } else { find_table(spec, outline, open, close)? };
    let data = Data::new(table, decimal);
    if !literal_gauge && data.rows == 0 {
        return Err(t!("The table has no rows").into());
    }
    let mut chart = Chart { spec, data: &data, kind, option: Map::new(), top: 0, bottom: 0 };
    chart.set("backgroundColor", json!("transparent"));
    chart.set("animation", json!(spec.flag("animation").unwrap_or(false)));
    if let Some(title) = spec.text("title") {
        let sub = spec.text("subtitle");
        chart.top = if sub.is_some() { 52 } else { 34 };
        chart.set("title", json!({ "text": title, "subtext": sub, "left": "center" }));
    }
    if let Some(colors) = palette(spec)? {
        chart.set("color", json!(colors));
    }
    let mut height = DEFAULT_HEIGHT;
    match kind {
        "line" | "area" | "bar" => cartesian(&mut chart)?,
        "pie" | "donut" | "rose" => pie(&mut chart)?,
        "scatter" | "bubble" => scatter(&mut chart)?,
        "radar" => radar(&mut chart)?,
        "funnel" => funnel(&mut chart)?,
        "gauge" => {
            height = 300;
            gauge(&mut chart)?
        }
        "boxplot" => boxplot(&mut chart)?,
        "candlestick" => candlestick(&mut chart)?,
        "heatmap" => heatmap(&mut chart)?,
        "calendar" => height = calendar(&mut chart)?,
        "sankey" | "graph" => network(&mut chart)?,
        "treemap" | "sunburst" | "tree" => {
            if let Some(h) = hierarchy(&mut chart)? {
                height = h;
            }
        }
        "parallel" => parallel(&mut chart)?,
        "themeriver" => theme_river(&mut chart)?,
        _ => unreachable!("chart type {kind}"),
    }
    let mut option = Json::Object(std::mem::take(&mut chart.option));
    strip_nulls(&mut option);
    if let Some(extra) = spec.get("echarts") {
        merge(&mut option, &yaml_to_json(extra));
    }
    let height = match spec.get("height") {
        Some(Yaml::Number(n)) => format!("{}px", n.as_f64().unwrap_or(height as f64)),
        Some(Yaml::String(s)) if s.trim().bytes().all(|b| b.is_ascii_digit()) => format!("{}px", s.trim()),
        Some(Yaml::String(s)) => s.trim().to_owned(),
        _ => format!("{height}px"),
    };
    let theme = spec.text("theme").map(|t| t.to_lowercase()).filter(|t| t == "light" || t == "dark");
    Ok(json!({ "option": option, "height": height, "theme": theme.unwrap_or_else(|| "auto".into()) }))
}

fn palette(spec: &Spec) -> Result<Option<Vec<String>>, String> {
    let value = spec.get("palette").or_else(|| spec.get("colors"));
    match value {
        None => Ok(None),
        Some(Yaml::Sequence(seq)) => Ok(Some(seq.iter().filter_map(yaml_text).collect())),
        Some(v) => {
            let name = yaml_text(v).unwrap_or_default().to_lowercase();
            PALETTES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, colors)| Some(colors.iter().map(|c| c.to_string()).collect()))
                .ok_or_else(|| {
                    let names: Vec<&str> = PALETTES.iter().map(|(n, _)| *n).collect();
                    t!("Unknown palette \"{}\" (palettes: {})", name, names.join(", "))
                })
        }
    }
}

/// Removes `null` members of objects (not of arrays, where they mark gaps).
fn strip_nulls(v: &mut Json) {
    match v {
        Json::Object(m) => {
            m.retain(|_, v| !v.is_null());
            m.values_mut().for_each(strip_nulls);
        }
        Json::Array(a) => a.iter_mut().for_each(strip_nulls),
        _ => {}
    }
}

/// Merges `extra` into `base`: objects member by member, arrays element by
/// element, anything else replaces.
fn merge(base: &mut Json, extra: &Json) {
    match (base, extra) {
        (Json::Object(b), Json::Object(e)) => {
            for (k, v) in e {
                match b.get_mut(k) {
                    Some(old) => merge(old, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (Json::Array(b), Json::Array(e)) => {
            for (i, v) in e.iter().enumerate() {
                match b.get_mut(i) {
                    Some(old) => merge(old, v),
                    None => b.push(v.clone()),
                }
            }
        }
        (Json::Object(b), Json::Array(e)) if !b.is_empty() => {
            // `series: [...]` over a single object: merge the first.
            if let Some(first) = e.first() {
                let mut obj = Json::Object(std::mem::take(b));
                merge(&mut obj, first);
                if let Json::Object(m) = obj {
                    *b = m;
                }
            }
        }
        (b, e) => *b = e.clone(),
    }
}

/// A round number at or above `v` (for axis maxima).
fn nice_ceil(v: f64) -> f64 {
    if v <= 0.0 || !v.is_finite() {
        return v;
    }
    let exp = 10f64.powf(v.log10().floor());
    let f = v / exp;
    let nice = [1.0, 1.2, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0].into_iter().find(|n| *n >= f - 1e-9).unwrap_or(10.0);
    nice * exp
}

fn unit_template(unit: &str) -> String {
    match unit {
        "$" | "£" | "¥" => format!("{unit}{{value}}"),
        "%" => "{value}%".to_owned(),
        u => format!("{{value}} {u}"),
    }
}

fn line_or_bar(kind: &str) -> &'static str {
    if kind == "bar" {
        "bar"
    } else {
        "line"
    }
}

/// Line, area and bar charts (also stacked, horizontal, with a second axis).
fn cartesian(c: &mut Chart) -> Result<(), String> {
    let x = c.x()?;
    let group = c.opt_col("group")?;
    let horizontal = c.spec.flag("horizontal").unwrap_or(false);
    let stack = c.spec.flag("stack").unwrap_or(false);
    let series_type = line_or_bar(c.kind);
    let time = c.col(x).kind == Kind::Date && c.kind != "bar" && group.is_none() && !horizontal;
    let style = |c: &Chart, name: &str, kind: &str, data: Json, second: bool| {
        let label_pos = if horizontal { "right" } else { "top" };
        json!({
            "name": name,
            "type": kind,
            "data": data,
            "stack": if stack && !second { Some("total") } else { None },
            "smooth": c.spec.flag("smooth"),
            "step": if c.spec.flag("step").unwrap_or(false) { Some("middle") } else { None },
            "areaStyle": if c.kind == "area" || (kind == "line" && c.spec.flag("area").unwrap_or(false)) {
                Some(json!({ "opacity": if stack { 0.7 } else { 0.25 } }))
            } else {
                None
            },
            "showSymbol": if kind == "line" { Some(c.data.rows <= 60) } else { None },
            "label": { "show": c.labels_on(false), "position": label_pos },
            "emphasis": { "focus": "series" },
            if horizontal { "xAxisIndex" } else { "yAxisIndex" }: if second { 1 } else { 0 },
        })
    };
    let mut series = Vec::new();
    let categories: Vec<String>;
    let mut value_cols: Vec<usize>;
    let mut second_cols: Vec<usize> = Vec::new();
    if let Some(g) = group {
        let y = c.value(&[x, g])?;
        value_cols = vec![y];
        categories = c.distinct(x);
        for name in c.distinct(g) {
            let mut data = vec![Json::Null; categories.len()];
            for r in 0..c.data.rows {
                if c.col(g).label(r) == name {
                    if let Some(i) = categories.iter().position(|cat| *cat == c.col(x).label(r)) {
                        data[i] = c.num(y, r);
                    }
                }
            }
            series.push(style(c, &name, series_type, json!(data), false));
        }
    } else {
        value_cols = c.numbers("y", &[x])?;
        if let Some(names) = c.spec.names("y2") {
            second_cols = names.iter().map(|n| c.data.find(n)).collect::<Result<_, _>>()?;
            value_cols.retain(|v| !second_cols.contains(v));
        }
        let order = c.order(value_cols.first().copied().unwrap_or(x));
        categories = order.iter().map(|&r| c.col(x).label(r)).collect();
        for (cols, second) in [(&value_cols, false), (&second_cols, true)] {
            for &col in cols {
                let data: Vec<Json> = if time {
                    order.iter().map(|&r| json!([c.cell(x, r), c.num(col, r)])).collect()
                } else {
                    order.iter().map(|&r| c.num(col, r)).collect()
                };
                let kind = if second && c.kind == "bar" { "line" } else { series_type };
                series.push(style(c, &c.col(col).name, kind, json!(data), second));
            }
        }
    }
    let count = series.len();
    if c.labels_on(false) && !horizontal {
        c.top += 14;
    }
    c.set("series", json!(series));
    c.set("tooltip", json!({ "trigger": "axis", "axisPointer": { "type": if series_type == "bar" { "shadow" } else { "line" } } }));
    c.legend(count > 1);
    c.zoom(horizontal, false);
    let category_axis = if time {
        json!({ "type": "time" })
    } else {
        json!({ "type": "category", "data": categories, "boundaryGap": series_type == "bar" })
    };
    let mut value_axes = vec![c.value_axis(&value_cols)];
    if !second_cols.is_empty() {
        let mut axis = c.value_axis(&second_cols);
        axis["splitLine"] = json!({ "show": false });
        axis["axisLabel"] = json!({ "formatter": c.column_unit(&second_cols).map(|u| unit_template(&u)) });
        value_axes.push(axis);
    }
    let (xa, ya) = if horizontal { (json!(value_axes), category_axis) } else { (category_axis, json!(value_axes)) };
    c.set("xAxis", xa);
    c.set("yAxis", ya);
    if horizontal {
        c.option["yAxis"]["inverse"] = json!(true);
    }
    let grid = c.grid();
    c.set("grid", grid);
    Ok(())
}

/// Pie, donut and rose charts; several value columns give several pies.
fn pie(c: &mut Chart) -> Result<(), String> {
    let name = match c.opt_col("x")?.or(c.opt_col("name")?) {
        Some(n) => n,
        None => c.data.first_of(Kind::Text, &[]).unwrap_or(0),
    };
    let values = match c.spec.names("value") {
        Some(_) => vec![c.value(&[name])?],
        None => c.numbers("y", &[name])?,
    };
    let n = values.len();
    let round = |v: f64| (v * 10.0).round() / 10.0;
    let outer = if n == 1 { 70.0 } else { round((95.0 / n as f64).min(60.0)) };
    let inner = match c.kind {
        "donut" => round(outer * 0.55),
        _ => 0.0,
    };
    let order = c.order(values[0]);
    let labels = c.labels_on(true);
    let mut series = Vec::new();
    let mut titles = Vec::new();
    for (i, &col) in values.iter().enumerate() {
        let data: Vec<Json> = order
            .iter()
            .filter_map(|&r| {
                let v = c.col(col).num(r).filter(|v| *v >= 0.0)?;
                Some(json!({ "name": c.col(name).label(r), "value": v }))
            })
            .collect();
        let cx = round((2 * i + 1) as f64 * 50.0 / n as f64);
        let cy = if n > 1 { 50.0 } else { 55.0 };
        series.push(json!({
            "type": "pie",
            "name": c.col(col).name,
            "data": data,
            "center": [format!("{cx}%"), format!("{cy}%")],
            "radius": [format!("{inner}%"), format!("{outer}%")],
            "roseType": if c.kind == "rose" { Some("radius") } else { None },
            "itemStyle": if c.kind == "donut" { Some(json!({ "borderRadius": 6, "borderColor": "rgba(128,128,128,.25)", "borderWidth": 1 })) } else { None },
            "label": { "show": labels && n <= 2, "formatter": "{b}: {d}%" },
            "emphasis": { "label": { "show": true, "fontWeight": "bold" } },
        }));
        if n > 1 {
            titles.push(json!({
                "text": c.col(col).name,
                "left": format!("{cx}%"),
                "top": format!("{}%", cy + outer / 2.0 + 8.0),
                "textAlign": "center",
                "textStyle": { "fontSize": 13, "fontWeight": "normal" },
            }));
        }
    }
    if !titles.is_empty() {
        let mut all: Vec<Json> = c.option.remove("title").into_iter().collect();
        all.extend(titles);
        c.set("title", json!(all));
    }
    c.set("series", json!(series));
    c.set("tooltip", json!({ "trigger": "item", "formatter": "{a}<br/>{b}: {c} ({d}%)" }));
    c.legend(n > 2 || !labels);
    Ok(())
}

/// Scatter and bubble charts; `group` splits the points into series.
fn scatter(c: &mut Chart) -> Result<(), String> {
    let x = match c.opt_col("x")? {
        Some(x) => x,
        None => c.data.first_of(Kind::Number, &[]).or(c.data.first_of(Kind::Date, &[])).unwrap_or(0),
    };
    if !matches!(c.col(x).kind, Kind::Number | Kind::Date) {
        return Err(t!("Column \"{}\" does not hold numbers", c.col(x).name));
    }
    let size = c.opt_col("size")?;
    let group = c.opt_col("group")?;
    let label = match c.opt_col("label")? {
        Some(l) => Some(l),
        None => c.data.first_of(Kind::Text, &[x, group.unwrap_or(usize::MAX)]),
    };
    let size = match size {
        None if c.kind == "bubble" => {
            // The first number column after the y column.
            let mut except: Vec<usize> = [Some(x), group, label].into_iter().flatten().collect();
            except.push(c.numbers("y", &except)?[0]);
            Some(c.data.first_of(Kind::Number, &except).ok_or_else(|| t!("Bubble charts need a `size` column"))?)
        }
        s => s,
    };
    let except: Vec<usize> = [Some(x), size, group, label].into_iter().flatten().collect();
    let ys = c.numbers("y", &except)?;
    let max_size = size.map_or(1.0, |s| (0..c.data.rows).filter_map(|r| c.col(s).num(r)).fold(0.0, f64::max).max(1e-9));
    let point = |c: &Chart, r: usize, y: usize| -> Option<Json> {
        c.col(y).num(r)?;
        let mut value = vec![c.cell(x, r), c.num(y, r)];
        let mut item = Map::new();
        if let Some(s) = size {
            let v = c.col(s).num(r).unwrap_or(0.0).max(0.0);
            value.push(json!(v));
            item.insert("symbolSize".into(), json!(((6.0 + 44.0 * (v / max_size).sqrt()) * 10.0).round() / 10.0));
        }
        if let Some(l) = label {
            item.insert("name".into(), json!(c.col(l).label(r)));
        }
        item.insert("value".into(), json!(value));
        Some(Json::Object(item))
    };
    let mut series = Vec::new();
    let emphasis = json!({ "focus": "series" });
    let item_style = if size.is_some() { json!({ "opacity": 0.75 }) } else { json!({}) };
    let label_cfg = json!({ "show": c.labels_on(false), "formatter": "{b}", "position": "top" });
    if let Some(g) = group {
        let y = ys[0];
        for name in c.distinct(g) {
            let data: Vec<Json> = (0..c.data.rows).filter(|&r| c.col(g).label(r) == name).filter_map(|r| point(c, r, y)).collect();
            series.push(json!({ "type": "scatter", "name": name, "data": data, "emphasis": emphasis, "itemStyle": item_style, "label": label_cfg }));
        }
    } else {
        for &y in &ys {
            let data: Vec<Json> = (0..c.data.rows).filter_map(|r| point(c, r, y)).collect();
            series.push(json!({ "type": "scatter", "name": c.col(y).name, "data": data, "emphasis": emphasis, "itemStyle": item_style, "label": label_cfg }));
        }
    }
    let count = series.len();
    c.set("series", json!(series));
    c.set("tooltip", json!({ "trigger": "item" }));
    c.legend(count > 1);
    let xtype = if c.col(x).kind == Kind::Date { "time" } else { "value" };
    let xname = c.col(x).name.clone();
    c.set("xAxis", json!({ "type": xtype, "scale": true, "name": xname, "nameLocation": "middle", "nameGap": 28, "axisLabel": { "formatter": c.unit_format(&[x]) } }));
    let mut yaxis = c.value_axis(&ys);
    yaxis["scale"] = json!(true);
    if ys.len() == 1 {
        yaxis["name"] = json!(c.col(ys[0]).name);
    }
    c.set("yAxis", yaxis);
    c.bottom += 24;
    let grid = c.grid();
    c.set("grid", grid);
    Ok(())
}

/// Radar chart: the rows are the axes, every value column a shape.
fn radar(c: &mut Chart) -> Result<(), String> {
    let x = c.x()?;
    let ys = c.numbers("y", &[x])?;
    let max = c.spec.number("max").unwrap_or_else(|| {
        let chart: &Chart = c;
        nice_ceil(ys.iter().flat_map(|&y| (0..chart.data.rows).filter_map(move |r| chart.col(y).num(r))).fold(0.0, f64::max))
    });
    let min = c.spec.number("min");
    let indicator: Vec<Json> = c.labels(x).into_iter().map(|name| json!({ "name": name, "max": max, "min": min })).collect();
    let fill = c.spec.flag("area").unwrap_or(true);
    let data: Vec<Json> = ys
        .iter()
        .map(|&y| {
            json!({
                "name": c.col(y).name,
                "value": c.values(y),
                "areaStyle": if fill { Some(json!({ "opacity": 0.15 })) } else { None },
                "label": { "show": c.labels_on(false) },
            })
        })
        .collect();
    c.legend(ys.len() > 1);
    let cy = 50 + (c.top as i32 - c.bottom as i32) / 8;
    let cy = if c.top > 0 { cy + 4 } else { cy };
    c.set("radar", json!({ "indicator": indicator, "center": ["50%", format!("{cy}%")], "radius": "58%" }));
    c.set("series", json!([{ "type": "radar", "data": data, "emphasis": { "focus": "self" } }]));
    c.set("tooltip", json!({ "trigger": "item" }));
    Ok(())
}

fn funnel(c: &mut Chart) -> Result<(), String> {
    let name = match c.opt_col("x")?.or(c.opt_col("name")?) {
        Some(n) => n,
        None => c.data.first_of(Kind::Text, &[]).unwrap_or(0),
    };
    let value = c.value(&[name])?;
    let data: Vec<Json> = (0..c.data.rows)
        .filter_map(|r| Some(json!({ "name": c.col(name).label(r), "value": c.col(value).num(r)? })))
        .collect();
    let sort = match c.spec.text("sort").map(|s| s.to_lowercase()).as_deref() {
        Some("asc" | "ascending") => "ascending",
        Some("none" | "false" | "no") => "none",
        _ => "descending",
    };
    c.legend(false);
    let top = c.top + 16;
    let bottom = c.bottom + 12;
    c.set("series", json!([{
        "type": "funnel",
        "name": c.col(value).name,
        "data": data,
        "sort": sort,
        "left": "10%", "width": "80%", "top": top, "bottom": bottom,
        "gap": 2,
        "label": { "show": c.labels_on(true), "position": "inside", "formatter": "{b}: {c}" },
        "emphasis": { "label": { "fontSize": 16 } },
    }]));
    c.set("tooltip", json!({ "trigger": "item", "formatter": "{b}: {c}" }));
    Ok(())
}

/// Gauges: one per table row (or a fixed `value:`).
fn gauge(c: &mut Chart) -> Result<(), String> {
    let mut items: Vec<(String, f64)> = Vec::new();
    let mut unit_cols = Vec::new();
    if let Some(Yaml::Number(v)) = c.spec.get("value") {
        items.push((c.spec.text("name").unwrap_or_default(), v.as_f64().unwrap_or(0.0)));
    } else {
        let name = match c.opt_col("x")?.or(c.opt_col("name")?) {
            Some(n) => Some(n),
            None => c.data.first_of(Kind::Text, &[]),
        };
        let value = c.value(&[name.unwrap_or(usize::MAX)])?;
        unit_cols.push(value);
        for r in 0..c.data.rows {
            if let Some(v) = c.col(value).num(r) {
                items.push((name.map(|n| c.col(n).label(r)).unwrap_or_default(), v));
            }
        }
        items.truncate(8);
    }
    let min = c.spec.number("min").unwrap_or(0.0);
    let biggest = items.iter().map(|i| i.1).fold(0.0, f64::max);
    let max = c.spec.number("max").unwrap_or(if biggest <= 100.0 { 100.0 } else { nice_ceil(biggest) });
    let unit = match c.spec.text("unit") {
        Some(u) => Some(u),
        None => unit_cols.first().and_then(|&u| c.col(u).unit.clone()),
    };
    let formatter = match unit.as_deref() {
        Some("%") => "{value}%".to_owned(),
        Some(u) => format!("{{value}} {u}"),
        None => "{value}".to_owned(),
    };
    let n = items.len().max(1);
    let radius = if n <= 2 { 80.0 } else { (180.0 / n as f64).min(60.0).round() };
    let small = n > 3;
    let series: Vec<Json> = items
        .iter()
        .enumerate()
        .map(|(i, (name, v))| {
            let cx = (2 * i + 1) as f64 * 50.0 / n as f64;
            json!({
                "type": "gauge",
                "center": [format!("{cx}%"), if n == 1 { "60%" } else { "58%" }],
                "radius": format!("{radius}%"),
                "min": min,
                "max": max,
                "progress": { "show": true, "width": if small { 8 } else { 12 } },
                "axisLine": { "lineStyle": { "width": if small { 8 } else { 12 } } },
                "axisLabel": { "show": n <= 2, "distance": 16, "fontSize": 11 },
                "splitNumber": if n == 1 { 10 } else { 5 },
                "splitLine": { "length": if small { 6 } else { 10 }, "distance": 2 },
                "axisTick": { "show": !small, "distance": 2 },
                "pointer": { "width": if small { 4 } else { 6 } },
                "title": { "offsetCenter": [0, "78%"], "fontSize": if small { 12 } else { 14 } },
                "detail": { "formatter": formatter, "offsetCenter": [0, "48%"], "fontSize": if small { 16 } else { 22 }, "valueAnimation": false },
                "data": [{ "value": v, "name": name }],
            })
        })
        .collect();
    c.set("series", json!(series));
    c.set("tooltip", json!({ "trigger": "item", "formatter": "{b}: {c}" }));
    Ok(())
}

/// Quartiles and whiskers (Tukey) of samples: low, q1, median, q3, high
/// and the outliers.
fn box_stats(mut v: Vec<f64>) -> Option<([f64; 5], Vec<f64>)> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let q = |p: f64| {
        let pos = p * (v.len() - 1) as f64;
        let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
        v[lo] + (v[hi] - v[lo]) * (pos - lo as f64)
    };
    let (q1, med, q3) = (q(0.25), q(0.5), q(0.75));
    let iqr = q3 - q1;
    let (lo_fence, hi_fence) = (q1 - 1.5 * iqr, q3 + 1.5 * iqr);
    let low = v.iter().copied().find(|x| *x >= lo_fence).unwrap_or(q1);
    let high = v.iter().rev().copied().find(|x| *x <= hi_fence).unwrap_or(q3);
    let outliers = v.iter().copied().filter(|x| *x < lo_fence || *x > hi_fence).collect();
    Some(([low, q1, med, q3, high], outliers))
}

/// Box plots: one box per number column, or per group (`x`) of a value column.
fn boxplot(c: &mut Chart) -> Result<(), String> {
    let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
    let x = c.opt_col("x")?.or(c.opt_col("group")?);
    let mut unit_cols = Vec::new();
    match x.filter(|&x| c.col(x).kind != Kind::Number) {
        Some(x) => {
            let value = c.value(&[x])?;
            unit_cols.push(value);
            for name in c.distinct(x) {
                let samples = (0..c.data.rows).filter(|&r| c.col(x).label(r) == name).filter_map(|r| c.col(value).num(r)).collect();
                groups.push((name, samples));
            }
        }
        None => {
            for col in c.numbers("y", &[])? {
                unit_cols.push(col);
                groups.push((c.col(col).name.clone(), (0..c.data.rows).filter_map(|r| c.col(col).num(r)).collect()));
            }
        }
    }
    let horizontal = c.spec.flag("horizontal").unwrap_or(false);
    let mut boxes = Vec::new();
    let mut outliers = Vec::new();
    for (i, (_, samples)) in groups.iter().enumerate() {
        match box_stats(samples.clone()) {
            Some((stats, outs)) => {
                boxes.push(json!(stats));
                for o in outs {
                    outliers.push(if horizontal { json!([o, i]) } else { json!([i, o]) });
                }
            }
            None => boxes.push(json!([])),
        }
    }
    let names: Vec<&str> = groups.iter().map(|g| g.0.as_str()).collect();
    c.set("series", json!([
        { "type": "boxplot", "name": "boxplot", "data": boxes, "colorBy": "data" },
        { "type": "scatter", "name": "outliers", "data": outliers },
    ]));
    c.set("tooltip", json!({ "trigger": "item" }));
    let category = json!({ "type": "category", "data": names, "boundaryGap": true });
    let mut value = c.value_axis(&unit_cols);
    value["scale"] = json!(true);
    let (xa, ya) = if horizontal { (value, category) } else { (category, value) };
    c.set("xAxis", xa);
    c.set("yAxis", ya);
    let grid = c.grid();
    c.set("grid", grid);
    Ok(())
}

const OPEN_NAMES: &[&str] = &["open", "opening", "o", "eröffnung", "eröffnungskurs", "start", "anfang", "erster"];
const HIGH_NAMES: &[&str] = &["high", "h", "max", "maximum", "hoch", "höchst", "höchstkurs", "tageshoch"];
const LOW_NAMES: &[&str] = &["low", "l", "min", "minimum", "tief", "tiefst", "tiefstkurs", "tagestief"];
const CLOSE_NAMES: &[&str] = &["close", "closing", "c", "schluss", "schlusskurs", "end", "ende", "letzter"];

/// Candlestick (OHLC) charts.
fn candlestick(c: &mut Chart) -> Result<(), String> {
    let x = c.x()?;
    let by_name = |c: &Chart, key: &str, names: &[&str]| -> Result<Option<usize>, String> {
        if let Some(col) = c.opt_col(key)? {
            return Ok(Some(col));
        }
        Ok((0..c.data.cols.len()).find(|&i| i != x && names.contains(&c.col(i).name.trim().to_lowercase().as_str())))
    };
    let named = [
        by_name(c, "open", OPEN_NAMES)?,
        by_name(c, "high", HIGH_NAMES)?,
        by_name(c, "low", LOW_NAMES)?,
        by_name(c, "close", CLOSE_NAMES)?,
    ];
    let [open, high, low, close] = if named.iter().all(Option::is_some) {
        named.map(Option::unwrap_or_default)
    } else {
        // In table order: open, high, low, close.
        let nums: Vec<usize> = (0..c.data.cols.len()).filter(|&i| i != x && c.col(i).kind == Kind::Number).collect();
        if nums.len() < 4 {
            return Err(t!("Candlestick charts need columns for open, high, low and close").into());
        }
        [nums[0], nums[1], nums[2], nums[3]]
    };
    let data: Vec<Json> = (0..c.data.rows).map(|r| json!([c.num(open, r), c.num(close, r), c.num(low, r), c.num(high, r)])).collect();
    let name = c.spec.text("title").unwrap_or_else(|| "OHLC".into());
    // Rising green, falling red (the ECharts default is the other way round).
    let style = json!({ "color": "#26a269", "color0": "#e01b24", "borderColor": "#26a269", "borderColor0": "#e01b24" });
    c.set("series", json!([{ "type": "candlestick", "name": name, "data": data, "itemStyle": style }]));
    c.set("tooltip", json!({ "trigger": "axis", "axisPointer": { "type": "cross" } }));
    let rows = c.data.rows;
    c.zoom(false, rows > 60);
    c.set("xAxis", json!({ "type": "category", "data": c.labels(x), "boundaryGap": true }));
    let mut y = c.value_axis(&[open, close]);
    y["scale"] = json!(true);
    c.set("yAxis", y);
    let grid = c.grid();
    c.set("grid", grid);
    Ok(())
}

/// Heat maps: a matrix (row labels in the first column, a column per x
/// value) or, with `value:`, x, y and value columns.
fn heatmap(c: &mut Chart) -> Result<(), String> {
    let mut cells: Vec<Json> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    let (xs, ys, unit_col) = if c.spec.get("value").is_some() {
        let x = c.x()?;
        let y = c.opt_col("y")?.or(c.data.first_of(Kind::Text, &[x])).ok_or_else(|| t!("Heat maps need a `y` column"))?;
        let v = c.value(&[x, y])?;
        let (xs, ys) = (c.distinct(x), c.distinct(y));
        for r in 0..c.data.rows {
            let Some(val) = c.col(v).num(r) else { continue };
            let xi = xs.iter().position(|l| *l == c.col(x).label(r)).unwrap_or(0);
            let yi = ys.iter().position(|l| *l == c.col(y).label(r)).unwrap_or(0);
            cells.push(json!([xi, yi, val]));
            values.push(val);
        }
        (xs, ys, v)
    } else {
        let rows = c.x()?;
        let cols = c.numbers("y", &[rows])?;
        for (xi, &col) in cols.iter().enumerate() {
            for r in 0..c.data.rows {
                if let Some(val) = c.col(col).num(r) {
                    cells.push(json!([xi, r, val]));
                    values.push(val);
                }
            }
        }
        (cols.iter().map(|&i| c.col(i).name.clone()).collect(), c.labels(rows), cols[0])
    };
    let min = c.spec.number("min").unwrap_or_else(|| values.iter().copied().fold(f64::INFINITY, f64::min));
    let max = c.spec.number("max").unwrap_or_else(|| values.iter().copied().fold(f64::NEG_INFINITY, f64::max));
    let small = cells.len() <= 120;
    c.set("series", json!([{
        "type": "heatmap",
        "name": c.col(unit_col).name,
        "data": cells,
        "label": { "show": c.labels_on(small) },
        "emphasis": { "itemStyle": { "shadowBlur": 8, "shadowColor": "rgba(0,0,0,.4)" } },
    }]));
    c.set("tooltip", json!({ "position": "top" }));
    let mut vm = visual_map(c, min, max);
    vm["orient"] = json!("vertical");
    vm["left"] = json!(null);
    vm["bottom"] = json!(null);
    vm["right"] = json!(0);
    vm["top"] = json!("middle");
    vm["itemHeight"] = json!(120);
    strip_nulls(&mut vm);
    c.set("visualMap", vm);
    c.set("xAxis", json!({ "type": "category", "data": xs, "splitArea": { "show": true } }));
    c.set("yAxis", json!({ "type": "category", "data": ys, "splitArea": { "show": true }, "inverse": true }));
    let mut grid = c.grid();
    grid["right"] = json!(72);
    c.set("grid", grid);
    Ok(())
}

fn visual_map(c: &Chart, min: f64, max: f64) -> Json {
    let (min, max) = if min.is_finite() && max.is_finite() { (min, max) } else { (0.0, 1.0) };
    let colors = match c.spec.get("colors").or_else(|| c.spec.get("palette")) {
        Some(Yaml::Sequence(seq)) => Some(seq.iter().filter_map(yaml_text).collect::<Vec<_>>()),
        _ => None,
    };
    json!({
        "min": min,
        "max": if max > min { max } else { min + 1.0 },
        "calculable": true,
        "orient": "horizontal",
        "left": "center",
        "bottom": 0,
        "inRange": colors.map(|c| json!({ "color": c })),
    })
}

/// Calendar heat map: one calendar per year of a date column.
fn calendar(c: &mut Chart) -> Result<u32, String> {
    let date = match c.opt_col("x")? {
        Some(d) => d,
        None => c.data.first_of(Kind::Date, &[]).ok_or_else(|| t!("Calendar charts need a column with dates"))?,
    };
    if c.col(date).kind != Kind::Date {
        return Err(t!("Column \"{}\" does not hold dates", c.col(date).name));
    }
    let value = c.value(&[date])?;
    let mut years: Vec<String> = Vec::new();
    let mut points: Vec<(String, f64)> = Vec::new();
    for r in 0..c.data.rows {
        let (Some(Value::Date(d)), Some(v)) = (c.col(date).values.get(r), c.col(value).num(r)) else { continue };
        let day = d[..10].to_owned();
        let year = day[..4].to_owned();
        if !years.contains(&year) {
            years.push(year);
        }
        points.push((day, v));
    }
    years.sort();
    let german = crate::i18n::lang() == crate::i18n::Lang::De;
    let (months, days): (Json, Json) = if german {
        (
            json!(["Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"]),
            json!(["So", "Mo", "Di", "Mi", "Do", "Fr", "Sa"]),
        )
    } else {
        (json!("EN"), json!("EN"))
    };
    const PER_YEAR: u32 = 150;
    let top = c.top + 34;
    let calendars: Vec<Json> = years
        .iter()
        .enumerate()
        .map(|(i, y)| {
            json!({
                "range": y,
                "top": top + i as u32 * PER_YEAR,
                "left": 44, "right": 16,
                "cellSize": ["auto", 15],
                "dayLabel": { "firstDay": 1, "nameMap": days },
                "monthLabel": { "nameMap": months },
                "yearLabel": { "show": years.len() > 1 || c.spec.get("title").is_none() },
                "itemStyle": { "borderWidth": 0.5 },
            })
        })
        .collect();
    let series: Vec<Json> = years
        .iter()
        .enumerate()
        .map(|(i, y)| {
            let data: Vec<Json> = points.iter().filter(|p| p.0.starts_with(y.as_str())).map(|p| json!([p.0, p.1])).collect();
            json!({ "type": "heatmap", "coordinateSystem": "calendar", "calendarIndex": i, "name": c.col(value).name, "data": data })
        })
        .collect();
    let min = c.spec.number("min").unwrap_or_else(|| points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min));
    let max = c.spec.number("max").unwrap_or_else(|| points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max));
    c.set("calendar", json!(calendars));
    c.set("series", json!(series));
    c.set("tooltip", json!({ "position": "top" }));
    c.set("visualMap", visual_map(c, min, max));
    Ok(top + years.len().max(1) as u32 * PER_YEAR + 40)
}

/// Sankey diagrams and network graphs from source, target and value columns.
fn network(c: &mut Chart) -> Result<(), String> {
    let source = match c.opt_col("source")? {
        Some(s) => s,
        None => c.data.first_of(Kind::Text, &[]).ok_or_else(|| t!("Sankey diagrams and graphs need source and target columns"))?,
    };
    let target = match c.opt_col("target")? {
        Some(t) => t,
        None => c.data.first_of(Kind::Text, &[source]).ok_or_else(|| t!("Sankey diagrams and graphs need source and target columns"))?,
    };
    let value = match c.opt_col("value")? {
        Some(v) => Some(v),
        None => c.data.first_of(Kind::Number, &[source, target]),
    };
    if c.kind == "sankey" && value.is_none() {
        return Err(t!("The table has no column with numbers").into());
    }
    let mut nodes: Vec<String> = Vec::new();
    let mut weight: Vec<f64> = Vec::new();
    let mut links = Vec::new();
    let mut widths = Vec::new();
    for r in 0..c.data.rows {
        let (s, t) = (c.col(source).label(r), c.col(target).label(r));
        if s.is_empty() || t.is_empty() {
            continue;
        }
        let v = value.and_then(|v| c.col(v).num(r));
        if c.kind == "sankey" && (v.is_none() || s == t) {
            continue;
        }
        for name in [&s, &t] {
            match nodes.iter().position(|n| n == name) {
                Some(i) => weight[i] += v.unwrap_or(1.0),
                None => {
                    nodes.push(name.clone());
                    weight.push(v.unwrap_or(1.0));
                }
            }
        }
        widths.push(v.unwrap_or(1.0));
        links.push(json!({ "source": s, "target": t, "value": v }));
    }
    if c.kind == "sankey" {
        let data: Vec<Json> = nodes.iter().map(|n| json!({ "name": n })).collect();
        let top = c.top + 16;
        c.set("series", json!([{
            "type": "sankey",
            "data": data,
            "links": links,
            "top": top, "bottom": 16, "left": 16, "right": 96,
            "emphasis": { "focus": "adjacency" },
            "lineStyle": { "color": "gradient", "curveness": 0.5 },
            "label": { "show": true },
        }]));
    } else {
        let max_w = weight.iter().copied().fold(0.0, f64::max).max(1e-9);
        let max_l = widths.iter().copied().fold(0.0, f64::max).max(1e-9);
        let data: Vec<Json> = nodes
            .iter()
            .zip(&weight)
            .map(|(n, w)| json!({ "name": n, "value": w, "symbolSize": (12.0 + 30.0 * (w / max_w).sqrt()).round() }))
            .collect();
        for (link, w) in links.iter_mut().zip(&widths) {
            link["lineStyle"] = json!({ "width": (1.0 + 5.0 * w / max_l).round() });
        }
        let directed = c.spec.flag("directed").unwrap_or(false);
        let circular = c.spec.text("layout").is_some_and(|l| l.eq_ignore_ascii_case("circular"));
        let top = c.top + 16;
        c.set("series", json!([{
            "type": "graph",
            "colorBy": "data",
            "layout": if circular { "circular" } else { "force" },
            "data": data,
            "links": links,
            "roam": true,
            "draggable": true,
            "top": top, "bottom": 16, "left": 16, "right": 16,
            "label": { "show": true, "position": "right" },
            "edgeSymbol": if directed { json!(["none", "arrow"]) } else { json!(["none", "none"]) },
            "edgeSymbolSize": 8,
            "force": { "repulsion": 220, "edgeLength": [50, 140], "gravity": 0.08, "layoutAnimation": false },
            "circular": { "rotateLabel": true },
            "lineStyle": { "color": "source", "curveness": 0.15, "opacity": 0.7 },
            "emphasis": { "focus": "adjacency" },
        }]));
    }
    c.set("tooltip", json!({ "trigger": "item" }));
    Ok(())
}

#[derive(Default)]
struct Node {
    name: String,
    value: Option<f64>,
    children: Vec<Node>,
}

impl Node {
    fn child(&mut self, name: &str) -> &mut Node {
        let i = match self.children.iter().position(|c| c.name == name) {
            Some(i) => i,
            None => {
                self.children.push(Node { name: name.to_owned(), ..Default::default() });
                self.children.len() - 1
            }
        };
        &mut self.children[i]
    }

    fn json(&self) -> Json {
        let mut m = Map::new();
        m.insert("name".into(), json!(self.name));
        if let Some(v) = self.value {
            m.insert("value".into(), json!(v));
        }
        if !self.children.is_empty() {
            m.insert("children".into(), json!(self.children.iter().map(Node::json).collect::<Vec<_>>()));
        }
        Json::Object(m)
    }

    fn leaves(&self) -> usize {
        if self.children.is_empty() {
            1
        } else {
            self.children.iter().map(Node::leaves).sum()
        }
    }

    fn depth(&self) -> usize {
        1 + self.children.iter().map(Node::depth).max().unwrap_or(0)
    }
}

/// Treemaps, sunbursts and trees: from path columns (Region | Country |
/// City | Value) or from name and parent columns. Returns the height a tree
/// needs.
fn hierarchy(c: &mut Chart) -> Result<Option<u32>, String> {
    let mut root = Node::default();
    if let Some(parent) = c.opt_col("parent")? {
        let name = match c.opt_col("name")?.or(c.opt_col("x")?) {
            Some(n) => n,
            None => c.data.first_of(Kind::Text, &[parent]).unwrap_or(0),
        };
        let value = match c.opt_col("value")? {
            Some(v) => Some(v),
            None => c.data.first_of(Kind::Number, &[name, parent]),
        };
        let rows: Vec<(String, String, Option<f64>)> =
            (0..c.data.rows).map(|r| (c.col(name).label(r), c.col(parent).label(r), value.and_then(|v| c.col(v).num(r)))).collect();
        let names: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
        fn add(node: &mut Node, rows: &[(String, String, Option<f64>)], depth: usize) {
            if depth > 32 {
                return;
            }
            for (n, p, v) in rows {
                if *p == node.name && *n != node.name {
                    let mut child = Node { name: n.clone(), value: *v, children: Vec::new() };
                    add(&mut child, rows, depth + 1);
                    node.children.push(child);
                }
            }
        }
        for (n, p, v) in &rows {
            if p.is_empty() || !names.contains(&p.as_str()) {
                let mut top = Node { name: n.clone(), value: *v, children: Vec::new() };
                add(&mut top, &rows, 0);
                root.children.push(top);
            }
        }
    } else {
        let value = match c.opt_col("value")? {
            Some(v) => Some(v),
            None => match c.spec.names("y") {
                Some(_) => Some(c.numbers("y", &[])?[0]),
                None => (0..c.data.cols.len()).rev().find(|&i| c.col(i).kind == Kind::Number),
            },
        };
        let path = match c.spec.names("path") {
            Some(names) => names.iter().map(|n| c.data.find(n)).collect::<Result<Vec<_>, _>>()?,
            None => (0..c.data.cols.len()).filter(|&i| Some(i) != value && c.col(i).kind != Kind::Number).collect(),
        };
        if path.is_empty() {
            return Err(t!("The table has no columns for the hierarchy").into());
        }
        for r in 0..c.data.rows {
            let mut node = &mut root;
            for &p in &path {
                let label = c.col(p).label(r);
                if label.is_empty() {
                    break;
                }
                node = node.child(&label);
            }
            if let Some(v) = value.and_then(|v| c.col(v).num(r)) {
                node.value = Some(node.value.unwrap_or(0.0) + v);
            }
        }
    }
    if root.children.is_empty() {
        return Err(t!("The table has no rows").into());
    }
    let top = c.top + 8;
    c.set("tooltip", json!({ "trigger": "item" }));
    let labels = c.labels_on(true);
    match c.kind {
        "treemap" => {
            let data: Vec<Json> = root.children.iter().map(Node::json).collect();
            // Levels below the (invisible) root.
            let levels = root.depth() - 1;
            c.set("series", json!([{
                "type": "treemap",
                "name": "·",
                "data": data,
                "top": top, "left": 8, "right": 8, "bottom": if levels > 2 { 36 } else { 8 },
                "roam": false,
                "nodeClick": if levels > 2 { json!("zoomToNode") } else { json!(false) },
                "breadcrumb": { "show": levels > 2, "bottom": 4 },
                "label": { "show": labels },
                "upperLabel": { "show": levels > 1, "height": 22 },
                "levels": [
                    { "itemStyle": { "borderWidth": 0, "gapWidth": 3 }, "upperLabel": { "show": false } },
                    { "itemStyle": { "gapWidth": 1, "borderWidth": 2, "borderColor": "rgba(128,128,128,.35)" } },
                ],
            }]));
            Ok(None)
        }
        "sunburst" => {
            let data: Vec<Json> = root.children.iter().map(Node::json).collect();
            c.set("series", json!([{
                "type": "sunburst",
                "data": data,
                "center": ["50%", "55%"],
                "radius": ["12%", "88%"],
                "label": { "show": labels, "rotate": "radial", "minAngle": 6 },
                "emphasis": { "focus": "ancestor" },
                "itemStyle": { "borderWidth": 1, "borderColor": "rgba(128,128,128,.4)" },
            }]));
            Ok(None)
        }
        _ => {
            let root = if root.children.len() == 1 {
                root.children.pop().unwrap_or_default()
            } else {
                root.name = c.spec.text("name").unwrap_or_else(|| "·".into());
                root
            };
            let vertical = c.spec.flag("horizontal") == Some(false);
            let radial = c.spec.text("layout").is_some_and(|l| l.eq_ignore_ascii_case("radial"));
            let leaves = root.leaves() as u32;
            let height = if radial {
                (leaves * 12 + 200).clamp(360, 900)
            } else if vertical {
                (root.depth() as u32 * 110 + 60).clamp(280, 1200)
            } else {
                (leaves * 26 + top + 40).clamp(240, 1600)
            };
            let (label, leaf_label) = if vertical {
                (json!({ "position": "top", "align": "center", "verticalAlign": "bottom" }), json!({ "position": "bottom", "align": "center", "verticalAlign": "top" }))
            } else {
                (json!({ "position": "left", "align": "right", "verticalAlign": "middle" }), json!({ "position": "right", "align": "left", "verticalAlign": "middle" }))
            };
            c.set("series", json!([{
                "type": "tree",
                "data": [root.json()],
                "layout": if radial { "radial" } else { "orthogonal" },
                "orient": if vertical { "TB" } else { "LR" },
                "top": top + 12, "bottom": 24, "left": if vertical { "8%" } else { "12%" }, "right": if vertical { "8%" } else { "22%" },
                "symbolSize": 8,
                "initialTreeDepth": -1,
                "expandAndCollapse": true,
                "label": label,
                "leaves": { "label": leaf_label },
                "emphasis": { "focus": "descendant" },
            }]));
            Ok(Some(height))
        }
    }
}

/// Parallel coordinates: one axis per column, one line per row.
fn parallel(c: &mut Chart) -> Result<(), String> {
    let group = c.opt_col("group")?;
    let axes: Vec<usize> = match c.spec.names("y") {
        Some(names) => names.iter().map(|n| c.data.find(n)).collect::<Result<_, _>>()?,
        None => (0..c.data.cols.len()).filter(|&i| Some(i) != group && c.col(i).kind != Kind::Empty).collect(),
    };
    let parallel_axis: Vec<Json> = axes
        .iter()
        .enumerate()
        .map(|(dim, &a)| match c.col(a).kind {
            Kind::Number => json!({ "dim": dim, "name": c.col(a).name, "type": "value", "scale": true }),
            Kind::Date => json!({ "dim": dim, "name": c.col(a).name, "type": "time" }),
            _ => json!({ "dim": dim, "name": c.col(a).name, "type": "category", "data": c.distinct(a) }),
        })
        .collect();
    let row = |c: &Chart, r: usize| -> Json {
        json!(axes.iter().map(|&a| if c.col(a).kind == Kind::Text { json!(c.col(a).label(r)) } else { c.cell(a, r) }).collect::<Vec<_>>())
    };
    let style = json!({ "width": 2, "opacity": 0.55 });
    let series: Vec<Json> = match group {
        Some(g) => c
            .distinct(g)
            .into_iter()
            .map(|name| {
                let data: Vec<Json> = (0..c.data.rows).filter(|&r| c.col(g).label(r) == name).map(|r| row(c, r)).collect();
                json!({ "type": "parallel", "name": name, "data": data, "lineStyle": style, "emphasis": { "lineStyle": { "width": 4, "opacity": 1 } } })
            })
            .collect(),
        None => {
            let data: Vec<Json> = (0..c.data.rows).map(|r| row(c, r)).collect();
            vec![json!({ "type": "parallel", "data": data, "lineStyle": style, "emphasis": { "lineStyle": { "width": 4, "opacity": 1 } } })]
        }
    };
    let count = series.len();
    c.legend(count > 1);
    c.set("parallelAxis", json!(parallel_axis));
    c.set("parallel", json!({ "left": 48, "right": 72, "top": c.top + 40, "bottom": c.bottom + 32 }));
    c.set("series", json!(series));
    c.set("tooltip", json!({ "trigger": "item" }));
    Ok(())
}

/// Theme rivers (stream graphs) over dates or numbers.
fn theme_river(c: &mut Chart) -> Result<(), String> {
    let x = c.x()?;
    let axis_type = match c.col(x).kind {
        Kind::Date => "time",
        Kind::Number => "value",
        _ => return Err(t!("Theme rivers need dates or numbers in column \"{}\"", c.col(x).name)),
    };
    let mut data = Vec::new();
    let mut count = 0;
    match c.opt_col("group")? {
        Some(g) => {
            let v = c.value(&[x, g])?;
            for r in 0..c.data.rows {
                data.push(json!([c.cell(x, r), c.col(v).num(r).unwrap_or(0.0), c.col(g).label(r)]));
            }
            count = c.distinct(g).len();
        }
        None => {
            for y in c.numbers("y", &[x])? {
                count += 1;
                for r in 0..c.data.rows {
                    data.push(json!([c.cell(x, r), c.col(y).num(r).unwrap_or(0.0), c.col(y).name]));
                }
            }
        }
    }
    c.legend(count > 1);
    c.set("singleAxis", json!({ "type": axis_type, "top": c.top + 24, "bottom": c.bottom + 36, "left": 24, "right": 24, "axisPointer": { "animation": false, "label": { "show": true } } }));
    c.set("series", json!([{ "type": "themeRiver", "data": data, "emphasis": { "itemStyle": { "shadowBlur": 12, "shadowColor": "rgba(0,0,0,.35)" } }, "label": { "show": c.labels_on(false) } }]));
    c.set("tooltip", json!({ "trigger": "axis", "axisPointer": { "type": "line" } }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The resolved JSON of the first chart in `md`.
    fn chart(md: &str) -> Json {
        let out = expand(md);
        let mut lines = out.lines();
        lines.find(|l| l.trim_end().ends_with(RESOLVED)).expect("resolved block");
        serde_json::from_str(lines.next().unwrap().trim()).unwrap()
    }

    fn error(md: &str) -> String {
        chart(md)["error"].as_str().unwrap_or_default().to_owned()
    }

    const SALES: &str = "# Sales\n\n| Month | Revenue | Costs |\n|---|---|---|\n| Jan | 1.200 € | 800 € |\n| Feb | 1.500,50 € | 900 € |\n| Mar | 1.100 € | - |\n\n";

    #[test]
    fn keeps_line_count_and_other_code() {
        let md = format!("{SALES}```chart\ntype: line\ntitle: Revenue\n```\n\n```rust\nlet chart = 1;\n```\n");
        let out = expand(&md);
        assert_eq!(out.lines().count(), md.lines().count());
        assert!(out.contains("```qn-chart\n{"));
        assert!(out.contains("let chart = 1;"));
        assert!(!out.contains("title: Revenue"));
        assert!(matches!(expand("no charts here"), Cow::Borrowed(_)));
        // Quoted blocks stay as they are.
        assert!(expand("> ```chart\n> type: bar\n> ```\n").contains("```chart"));
    }

    #[test]
    fn line_chart_from_table_above() {
        let j = chart(&format!("{SALES}```chart\ntype: line\ntitle: Revenue\n```\n"));
        let o = &j["option"];
        assert_eq!(o["xAxis"]["data"], json!(["Jan", "Feb", "Mar"]));
        assert_eq!(o["series"][0]["name"], "Revenue");
        assert_eq!(o["series"][0]["data"], json!([1200.0, 1500.5, 1100.0]));
        assert_eq!(o["series"][1]["data"], json!([800.0, 900.0, null]));
        assert_eq!(o["yAxis"][0]["axisLabel"]["formatter"], "{value} €");
        assert_eq!(o["title"]["text"], "Revenue");
        assert_eq!(o["legend"]["bottom"], 0);
        assert_eq!(j["height"], "360px");
        assert_eq!(j["theme"], "auto");
    }

    #[test]
    fn finds_tables() {
        let md = format!("```chart\ntype: bar\n```\n\n{SALES}## Other\n\n| a | b |\n|---|---|\n| x | 1 |\n\n```chart\ntable: Sales\ny: Costs\n```\n```chart\ntable: 2\n```\n");
        let out = expand(&md);
        let charts: Vec<Json> = out
            .lines()
            .zip(out.lines().skip(1))
            .filter(|(a, _)| a.ends_with(RESOLVED))
            .map(|(_, b)| serde_json::from_str(b).unwrap())
            .collect();
        // Nothing above the first chart: the table below.
        assert_eq!(charts[0]["option"]["series"][0]["name"], "Revenue");
        assert_eq!(charts[1]["option"]["series"][0]["name"], "Costs");
        assert_eq!(charts[2]["option"]["xAxis"]["data"], json!(["x"]));
        assert!(error("```chart\ntable: 3\n```\n").contains("no table 3"));
        assert!(error(&format!("{SALES}```chart\ntable: Nope\n```\n")).contains("No heading"));
    }

    #[test]
    fn front_matter_defaults_and_overrides() {
        let md = format!("---\ncharts:\n  palette: vintage\n  height: 200\n  theme: dark\n---\n{SALES}```chart\ntype: area\nstack: true\nheight: 250\necharts:\n  series:\n    - smooth: true\n```\n");
        let j = chart(&md);
        assert_eq!(j["height"], "250px");
        assert_eq!(j["theme"], "dark");
        assert_eq!(j["option"]["color"][0], "#d87c7c");
        assert_eq!(j["option"]["series"][0]["stack"], "total");
        assert_eq!(j["option"]["series"][0]["smooth"], true);
        assert!(j["option"]["series"][0]["areaStyle"].is_object());
    }

    #[test]
    fn errors() {
        assert!(error(&format!("{SALES}```chart\ntype: pyramid\n```\n")).contains("pyramid"));
        assert!(error(&format!("{SALES}```chart\ntitel: x\n```\n")).contains("titel"));
        assert!(error(&format!("{SALES}```chart\ny: Nope\n```\n")).contains("Month, Revenue, Costs"));
        assert!(error(&format!("{SALES}```chart\npalette: neon\n```\n")).contains("neon"));
        assert!(error("```chart\ntype: bar\n```\n").contains("No table"));
    }

    #[test]
    fn bars_grouped_and_secondary_axis() {
        let md = "| Year | Team | Points |\n|---|---|---|\n| 2025 | A | 3 |\n| 2025 | B | 5 |\n| 2026 | A | 4 |\n\n```chart\ntype: bar\ngroup: Team\nhorizontal: true\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["yAxis"]["data"], json!(["2025", "2026"]));
        assert_eq!(o["series"][0]["name"], "A");
        assert_eq!(o["series"][0]["data"], json!([3.0, 4.0]));
        assert_eq!(o["series"][1]["data"], json!([5.0, null]));
        let o = &chart(&format!("{SALES}```chart\ntype: bar\ny: Revenue\ny2: Costs\n```\n"))["option"];
        assert_eq!(o["series"][1]["type"], "line");
        assert_eq!(o["series"][1]["yAxisIndex"], 1);
        assert_eq!(o["yAxis"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn time_axis_for_dates() {
        let md = "| Day | Temp |\n|---|---|\n| 01.03.2026 | 5,5 |\n| 2026-03-02 | 7 |\n\n```chart\ntype: line\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["xAxis"]["type"], "time");
        assert_eq!(o["series"][0]["data"], json!([["2026-03-01", 5.5], ["2026-03-02", 7.0]]));
    }

    #[test]
    fn pies() {
        let o = &chart(&format!("{SALES}```chart\ntype: donut\ny: Revenue\nsort: asc\n```\n"))["option"];
        assert_eq!(o["series"][0]["data"][0], json!({ "name": "Mar", "value": 1100.0 }));
        assert_eq!(o["series"][0]["radius"][0], "38.5%");
        let o = &chart(&format!("{SALES}```chart\ntype: pie\n```\n"))["option"];
        assert_eq!(o["series"].as_array().unwrap().len(), 2);
        assert_eq!(o["title"][0]["text"], "Revenue");
    }

    #[test]
    fn scatter_and_bubbles() {
        let md = "| Land | GDP | Life | Pop |\n|---|---|---|---|\n| A | 10 | 70 | 100 |\n| B | 20 | 80 | 25 |\n\n```chart\ntype: bubble\nx: GDP\ny: Life\n```\n";
        let o = &chart(md)["option"];
        let p = &o["series"][0]["data"][0];
        assert_eq!(p["value"], json!([10.0, 70.0, 100.0]));
        assert_eq!(p["name"], "A");
        assert_eq!(p["symbolSize"], 50.0);
        assert_eq!(o["series"][0]["data"][1]["symbolSize"], 28.0);
    }

    #[test]
    fn radar_funnel_gauge() {
        let md = "| Skill | Ann | Bob |\n|---|---|---|\n| Rust | 8 | 6 |\n| SQL | 5 | 9 |\n\n```chart\ntype: radar\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["radar"]["indicator"][1], json!({ "name": "SQL", "max": 10.0 }));
        assert_eq!(o["series"][0]["data"][1]["value"], json!([6.0, 9.0]));
        let o = &chart(&format!("{SALES}```chart\ntype: funnel\n```\n"))["option"];
        assert_eq!(o["series"][0]["sort"], "descending");
        let o = &chart("```chart\ntype: gauge\nvalue: 72\nname: CPU\nunit: \"%\"\n```\n")["option"];
        assert_eq!(o["series"][0]["data"][0], json!({ "value": 72.0, "name": "CPU" }));
        assert_eq!(o["series"][0]["detail"]["formatter"], "{value}%");
        let o = &chart(&format!("{SALES}```chart\ntype: gauge\ny: Costs\n```\n"))["option"];
        assert_eq!(o["series"].as_array().unwrap().len(), 2);
        assert_eq!(o["series"][0]["max"], 1000.0);
    }

    #[test]
    fn boxplots() {
        assert_eq!(box_stats(vec![1.0, 2.0, 3.0, 4.0, 100.0]).unwrap(), ([1.0, 2.0, 3.0, 4.0, 4.0], vec![100.0]));
        let md = "| Group | Time |\n|---|---|\n| a | 1 |\n| a | 3 |\n| b | 2 |\n\n```chart\ntype: boxplot\nx: Group\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["xAxis"]["data"], json!(["a", "b"]));
        assert_eq!(o["series"][0]["data"][0], json!([1.0, 1.5, 2.0, 2.5, 3.0]));
    }

    #[test]
    fn candlesticks() {
        let md = "| Date | Open | Close | Low | High |\n|---|---|---|---|---|\n| 2026-01-02 | 10 | 12 | 9 | 13 |\n\n```chart\ntype: candlestick\n```\n";
        assert_eq!(chart(md)["option"]["series"][0]["data"][0], json!([10.0, 12.0, 9.0, 13.0]));
        let md = "| Day | A | B | C | D |\n|---|---|---|---|---|\n| Mo | 10 | 13 | 9 | 12 |\n\n```chart\ntype: ohlc\n```\n";
        assert_eq!(chart(md)["option"]["series"][0]["data"][0], json!([10.0, 12.0, 9.0, 13.0]));
    }

    #[test]
    fn heatmaps_and_calendar() {
        let md = "| Day | 8h | 12h |\n|---|---|---|\n| Mo | 1 | 5 |\n| Tu | 2 | 0 |\n\n```chart\ntype: heatmap\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["xAxis"]["data"], json!(["8h", "12h"]));
        assert_eq!(o["yAxis"]["data"], json!(["Mo", "Tu"]));
        assert!(o["series"][0]["data"].as_array().unwrap().contains(&json!([1, 0, 5.0])));
        assert_eq!(o["visualMap"]["max"], 5.0);
        let md = "| Date | Steps |\n|---|---|\n| 2025-12-31 | 4000 |\n| 2026-01-01 | 9000 |\n\n```chart\ntype: calendar\n```\n";
        let j = chart(md);
        assert_eq!(j["option"]["calendar"].as_array().unwrap().len(), 2);
        assert_eq!(j["option"]["series"][1]["data"], json!([["2026-01-01", 9000.0]]));
        assert_eq!(j["height"], "374px");
    }

    #[test]
    fn sankey_and_graph() {
        let md = "| From | To | Amount |\n|---|---|---|\n| Salary | Rent | 900 |\n| Salary | Food | 400 |\n\n```chart\ntype: sankey\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["series"][0]["data"], json!([{ "name": "Salary" }, { "name": "Rent" }, { "name": "Food" }]));
        assert_eq!(o["series"][0]["links"][1], json!({ "source": "Salary", "target": "Food", "value": 400.0 }));
        let md = "| A | B |\n|---|---|\n| x | y |\n| y | z |\n\n```chart\ntype: graph\ndirected: true\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["series"][0]["data"][1]["value"], 2.0);
        assert_eq!(o["series"][0]["edgeSymbol"][1], "arrow");
    }

    #[test]
    fn hierarchies() {
        let md = "| Region | Country | Sales |\n|---|---|---|\n| EU | DE | 5 |\n| EU | FR | 3 |\n| US | | 7 |\n\n```chart\ntype: treemap\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["series"][0]["data"][0], json!({ "name": "EU", "children": [{ "name": "DE", "value": 5.0 }, { "name": "FR", "value": 3.0 }] }));
        assert_eq!(o["series"][0]["data"][1], json!({ "name": "US", "value": 7.0 }));
        let md = "| Name | Parent |\n|---|---|\n| CEO | |\n| CTO | CEO |\n| Dev | CTO |\n\n```chart\ntype: tree\nparent: Parent\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["series"][0]["data"][0], json!({ "name": "CEO", "children": [{ "name": "CTO", "children": [{ "name": "Dev" }] }] }));
    }

    #[test]
    fn parallel_and_river() {
        let md = "| Car | PS | Price | Class |\n|---|---|---|---|\n| A | 100 | 20000 | small |\n| B | 300 | 60000 | big |\n\n```chart\ntype: parallel\ngroup: Car\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["parallelAxis"][2]["type"], "category");
        assert_eq!(o["series"][1]["data"], json!([[300.0, 60000.0, "big"]]));
        let md = "| Date | A | B |\n|---|---|---|\n| 2026-01-01 | 1 | 2 |\n\n```chart\ntype: river\n```\n";
        let o = &chart(md)["option"];
        assert_eq!(o["singleAxis"]["type"], "time");
        assert_eq!(o["series"][0]["data"][1], json!(["2026-01-01", 2.0, "B"]));
        assert!(error(&format!("{SALES}```chart\ntype: themeriver\n```\n")).contains("Month"));
    }

    #[test]
    fn templates_render() {
        for (kind, _) in MENU_COMMON.iter().chain(MENU_MORE) {
            let (md, line) = template(kind, true);
            let j = chart(&format!("{md}\n"));
            assert!(j.get("error").is_none(), "{kind}: {j}");
            assert!(md.lines().nth(line).unwrap().starts_with("title:"), "{kind}");
            let (md, line) = template(kind, false);
            assert!(md.lines().nth(line).unwrap().starts_with("title:"), "{kind}");
        }
        assert_eq!(MENU_COMMON.len() + MENU_MORE.len(), TYPES.len());
    }

    #[test]
    fn merges_echarts_options() {
        let mut base = json!({ "a": { "b": 1, "c": 2 }, "s": [{ "x": 1 }, { "x": 2 }] });
        merge(&mut base, &json!({ "a": { "c": 3 }, "s": [{ "y": 1 }] }));
        assert_eq!(base, json!({ "a": { "b": 1, "c": 3 }, "s": [{ "x": 1, "y": 1 }, { "x": 2 }] }));
    }

    #[test]
    fn nice_numbers() {
        assert_eq!(nice_ceil(9.0), 10.0);
        assert_eq!(nice_ceil(1234.0), 1500.0);
        assert_eq!(nice_ceil(0.0), 0.0);
    }

    #[test]
    fn every_type_renders() {
        let md = "| Name | Date | Group | A | B | C | D |\n|---|---|---|---|---|---|---|\n| x | 2026-01-01 | g | 1 | 2 | 3 | 4 |\n| y | 2026-01-02 | h | 2 | 3 | 4 | 5 |\n\n";
        for (t, _) in TYPES {
            let extra = match *t {
                "sankey" | "graph" => "source: Name\ntarget: Group\n",
                "calendar" | "themeriver" => "x: Date\n",
                _ => "",
            };
            let j = chart(&format!("{md}```chart\ntype: {t}\n{extra}```\n"));
            assert!(j.get("error").is_none(), "{t}: {j}");
            assert!(j["option"]["series"].is_array(), "{t}");
        }
    }
}

#[cfg(test)]
mod demo {
    /// Writes a demo page with every chart type to `$QN_CHART_DEMO` (manual check).
    #[test]
    #[ignore]
    fn write_demo() {
        let Ok(path) = std::env::var("QN_CHART_DEMO") else { return };
        let mut md = String::from("# Chart demo\n\n");
        for (kind, label) in super::MENU_COMMON.iter().chain(super::MENU_MORE) {
            md.push_str(&format!("## {label}\n\n{}\n\n", super::template(kind, true).0));
        }
        md.push_str("## Extras\n\n| Month | Revenue | Costs | Margin |\n|---|---|---|---|\n| Jan | 1.200 € | 800 € | 33 % |\n| Feb | 1.500,50 € | 900 € | 40 % |\n| Mar | 1.100 € | - | 13 % |\n\n```chart\ntype: bar\ny: [Revenue, Costs]\ny2: Margin\ntitle: Combo with second axis\nsubtitle: German numbers\nlabels: true\npalette: vintage\n```\n\n```chart\ntable: Extras\ntype: bar\nhorizontal: true\nstack: true\nlegend: top\n```\n\n```chart\ntype: nope\n```\n");
        let body = crate::markdown::to_html(&md);
        let page = crate::html::standalone("Chart demo", &body);
        std::fs::write(&path, &page).unwrap();
        std::fs::write(format!("{path}.dark.html"), page.replace("data-theme=\"light\"", "data-theme=\"dark\"")).unwrap();
    }
}
