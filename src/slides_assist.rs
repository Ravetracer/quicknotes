//! Context-sensitive help for presentations: what can be written at the
//! cursor (completion proposals) and a short reference for the cheat-sheet
//! popover. Keyword lists come from [`crate::slides`], so they stay in sync
//! with the parser.

use crate::slides::{STEP_ANIMATIONS, THEMES, TRANSITIONS};

/// Where the cursor is, syntactically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Context {
    /// Plain slide content.
    Slide,
    /// The words of a container line (`::: …`).
    Container,
    /// A key in the front matter.
    FrontMatterKey,
    /// A key of a slide setting (`<!-- key: value; … -->`).
    SettingKey,
    /// The value of a front matter key, a slide setting or a container
    /// option (`anim=`, `delay=`, …).
    Value(String),
    /// Options in the alt text of an image (`![…](…)`).
    Image,
}

/// One proposal: the text shown, the text inserted and a short explanation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub label: String,
    pub insert: String,
    pub info: String,
}

/// The context at the cursor and the word being typed there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assist {
    pub context: Context,
    /// Byte offset in the line where the typed word starts.
    pub start: usize,
    pub word: String,
}

impl Assist {
    /// Proposals matching the typed word; options already present on a
    /// container line are left out.
    pub fn proposals(&self, line: &str) -> Vec<Item> {
        let word = self.word.to_ascii_lowercase();
        let used = self.used_words(line);
        items(&self.context)
            .into_iter()
            .filter(|i| {
                let label = i.label.to_ascii_lowercase();
                label.starts_with(&word) && label != word && !used.contains(&label.trim_end_matches('=').to_owned())
            })
            .collect()
    }

    /// Options written elsewhere on a container line or in an image's alt text.
    fn used_words(&self, line: &str) -> Vec<String> {
        let (from, to) = match self.context {
            Context::Container => (line.find(":::").map_or(0, |i| i + 3), line.len()),
            Context::Image => {
                let from = line[..self.start].rfind("![").map_or(self.start, |i| i + 2);
                (from, line[self.start..].find(']').map_or(line.len(), |i| self.start + i))
            }
            _ => return Vec::new(),
        };
        let mut used = Vec::new();
        let mut offset = from;
        for token in line[from..to].split([' ', '\t']) {
            if !token.is_empty() && !(offset..=offset + token.len()).contains(&self.start) {
                used.push(token.split('=').next().unwrap_or_default().to_ascii_lowercase());
            }
            offset += token.len() + 1;
        }
        used
    }
}

/// Analyzes the text of the cursor line before the cursor.
pub fn analyze(before: &str, in_front_matter: bool) -> Assist {
    let word_at = |context: Context, from: usize| {
        let start = before[from..].rfind([' ', '\t']).map_or(from, |i| from + i + 1);
        Assist { context, start, word: before[start..].to_owned() }
    };

    if in_front_matter {
        return match before.find(':') {
            Some(colon) => word_at(Context::Value(before[..colon].trim().to_ascii_lowercase()), colon + 1),
            None => word_at(Context::FrontMatterKey, 0),
        };
    }

    if let Some(open) = before.rfind("<!--") {
        if !before[open..].contains("-->") {
            let from = before[open..].rfind(';').map_or(open + 4, |i| open + i + 1);
            return match before[from..].find(':') {
                Some(colon) => {
                    let key = before[from..from + colon].trim().to_ascii_lowercase();
                    word_at(Context::Value(key), from + colon + 1)
                }
                None => word_at(Context::SettingKey, from),
            };
        }
    }

    let trimmed = before.trim_start();
    if let Some(rest) = trimmed.strip_prefix(":::") {
        if rest.starts_with([' ', '\t']) {
            let assist = word_at(Context::Container, before.len() - rest.len());
            if let Some(eq) = assist.word.find('=') {
                let key = assist.word[..eq].to_ascii_lowercase();
                let start = assist.start + eq + 1;
                return Assist { context: Context::Value(key), start, word: before[start..].to_owned() };
            }
            return assist;
        }
    }

    if let Some(open) = before.rfind("![") {
        if !before[open..].contains(']') {
            return word_at(Context::Image, open + 2);
        }
    }

    word_at(Context::Slide, 0)
}

fn item(label: &str, info: &str) -> Item {
    Item { label: label.to_owned(), insert: label.to_owned(), info: info.to_owned() }
}

fn names(list: &[&str], info: &str) -> Vec<Item> {
    list.iter().map(|n| item(n, info)).collect()
}

fn speeds() -> Vec<Item> {
    vec![
        item("fast", t!("Speed: 300 ms")),
        item("normal", t!("Speed: 600 ms")),
        item("slow", t!("Speed: 1 s")),
    ]
}

/// Everything that can be written in a context.
pub fn items(context: &Context) -> Vec<Item> {
    match context {
        Context::Slide => Vec::new(),
        Context::Container => {
            let mut list = vec![
                item("step", t!("The whole block is one step")),
                item("steps", t!("Each list item or paragraph is its own step")),
                item("dim", t!("Dims earlier steps of the container")),
                item("place", t!("Positions the block on the slide")),
                item("top", t!("Places the block at the top")),
                item("middle", t!("Places the block in the middle")),
                item("bottom", t!("Places the block at the bottom")),
                item("left", t!("Aligns left (with place: left edge)")),
                item("center", t!("Centers the text or the placed block")),
                item("right", t!("Aligns right (with place: right edge)")),
                item("x=", t!("Horizontal position in % of the slide")),
                item("y=", t!("Vertical position in % of the slide")),
                item("w=", t!("Width in % of the slide")),
                item("columns", t!("Columns of equal width, ||| starts the next one")),
                item("1:1", t!("Columns with this width ratio")),
                item("2:1", t!("Columns with this width ratio")),
                item("1:2", t!("Columns with this width ratio")),
                item("1:1:1", t!("Columns with this width ratio")),
                item("box", t!("Draws a card around the block")),
                item("small", t!("Smaller text")),
                item("large", t!("Larger text")),
                item("muted", t!("Muted text color")),
                item("size=", t!("Relative font size, e.g. size=1.3")),
                item("delay=", t!("Waits before the animation, e.g. delay=200ms")),
            ];
            list.extend(names(STEP_ANIMATIONS, t!("Step animation")));
            list.extend(speeds());
            list
        }
        Context::FrontMatterKey => vec![
            item("presentation: true", t!("Opens the file as presentation")),
            item("title: ", t!("Title of the presentation")),
            item("theme: ", t!("Color theme")),
            item("aspect: ", t!("Slide format")),
            item("transition: ", t!("Default slide transition")),
            item("step: ", t!("Default animation of steps")),
            item("paginate: ", t!("Slide numbers")),
            item("header: ", t!("Text at the top of every slide")),
            item("footer: ", t!("Text at the bottom of every slide")),
            item("duration: ", t!("Talk length, e.g. 20m")),
            item("style: |", t!("Extra CSS for the slides")),
        ]
        .into_iter()
        .map(key_item)
        .collect(),
        Context::SettingKey => vec![
            item("transition: ", t!("Slide transition")),
            item("step: ", t!("Animation of the steps on this slide")),
            item("background: ", t!("Background color or gradient")),
            item("color: ", t!("Text color")),
            item("class: ", t!("lead (title slide), invert (swapped colors)")),
            item("align: ", t!("left, center, right")),
            item("valign: ", t!("top, middle, bottom")),
            item("theme: ", t!("Color theme of this slide")),
            item("header: ", t!("Text at the top")),
            item("footer: ", t!("Text at the bottom")),
            item("paginate: ", t!("Slide number on/off")),
        ]
        .into_iter()
        .map(key_item)
        .collect(),
        Context::Value(key) => match key.as_str() {
            "transition" => [names(TRANSITIONS, t!("Slide transition")), speeds()].concat(),
            "step" | "steps" | "anim" | "animation" => [names(STEP_ANIMATIONS, t!("Step animation")), speeds()].concat(),
            "theme" => names(THEMES, t!("Color theme")),
            "aspect" => names(&["16:9", "4:3", "16:10"], t!("Slide format")),
            "paginate" | "presentation" => names(&["true", "false"], ""),
            "align" => names(&["left", "center", "right"], t!("Horizontal alignment")),
            "valign" => names(&["top", "middle", "bottom"], t!("Vertical alignment")),
            "class" => vec![item("lead", t!("Centered title slide")), item("invert", t!("Swapped colors"))],
            "delay" | "dur" | "duration" => names(&["200ms", "400ms", "0.8s", "1s"], t!("Duration")),
            _ => Vec::new(),
        },
        Context::Image => vec![
            item("bg", t!("Background image filling the slide")),
            item("contain", t!("Background: fits the whole image in")),
            item("left", t!("Floats left; with bg: image on the left side")),
            item("right", t!("Floats right; with bg: image on the right side")),
            item("center", t!("Centered image")),
            item("w=", t!("Width, e.g. w=400 or w=50%")),
            item("h=", t!("Height, e.g. h=300")),
            item("40%", t!("With bg left/right: width of the image side")),
        ],
    }
}

/// `key: ` proposals are listed and matched by the key alone.
fn key_item(mut i: Item) -> Item {
    i.label = i.label.split(':').next().unwrap_or_default().to_owned();
    i
}

/// Heading and rows (`syntax`, `explanation`) of the cheat sheet for a
/// context. Consecutive items with the same explanation share one row.
pub fn cheat_sheet(context: &Context) -> (String, Vec<(String, String)>) {
    let title = match context {
        Context::Slide => t!("Presentation syntax").to_owned(),
        Context::Container => t!("Container options (::: …)").to_owned(),
        Context::FrontMatterKey => t!("Presentation settings (front matter)").to_owned(),
        Context::SettingKey => t!("Slide settings (<!-- key: value; … -->)").to_owned(),
        Context::Value(key) => t!("Values for {}", key),
        Context::Image => t!("Image options (![…](file))").to_owned(),
    };
    let list = match context {
        Context::Slide => vec![
            item("---", t!("Starts a new slide")),
            item("::: steps fade-up", t!("Container: steps, animations, layout – closed by :::")),
            item("|||", t!("Next column inside ::: columns")),
            item("<!-- transition: zoom; class: lead -->", t!("Settings of this slide")),
            item("![w=400](image.jpg)", t!("Image with size and alignment")),
            item("![bg](image.jpg)", t!("Background image")),
            item("???", t!("Presenter notes follow")),
        ],
        Context::FrontMatterKey | Context::SettingKey => {
            items(context).into_iter().map(|mut i| {
                i.label = i.insert.trim_end().to_owned();
                i
            }).collect()
        }
        _ => items(context),
    };
    let mut rows: Vec<(String, String)> = Vec::new();
    for i in list {
        match rows.last_mut() {
            Some((label, info)) if *info == i.info && !i.info.is_empty() => {
                label.push_str(", ");
                label.push_str(&i.label);
            }
            _ => rows.push((i.label, i.info)),
        }
    }
    (title, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(before: &str, line: &str, fm: bool) -> Vec<String> {
        analyze(before, fm).proposals(line).into_iter().map(|i| i.label).collect()
    }

    #[test]
    fn container_words() {
        let a = analyze("::: steps fa", false);
        assert_eq!(a.context, Context::Container);
        assert_eq!((a.start, a.word.as_str()), (10, "fa"));
        assert_eq!(labels("::: steps fa", "::: steps fa", false), ["fade", "fade-up", "fade-down", "fade-left", "fade-right", "fast"]);
        // Words already on the line are not offered again.
        assert!(!labels("::: steps ", "::: steps ", false).contains(&"steps".to_owned()));
        assert!(labels("::: steps ", "::: steps ", false).contains(&"step".to_owned()));
        // The closing line and plain text get nothing.
        assert_eq!(analyze(":::", false).context, Context::Slide);
        assert!(labels("Some text", "Some text", false).is_empty());
    }

    #[test]
    fn container_values() {
        let a = analyze("::: step anim=zo", false);
        assert_eq!(a.context, Context::Value("anim".into()));
        assert_eq!((a.start, a.word.as_str()), (14, "zo"));
        assert_eq!(labels("::: step anim=zo", "::: step anim=zo", false), ["zoom", "zoom-out"]);
    }

    #[test]
    fn settings() {
        assert_eq!(analyze("<!-- tra", false).context, Context::SettingKey);
        assert_eq!(labels("<!-- tra", "<!-- tra", false), ["transition"]);
        let a = analyze("<!-- class: lead; transition: cu", false);
        assert_eq!(a.context, Context::Value("transition".into()));
        assert_eq!(a.word, "cu");
        assert_eq!(analyze("<!-- a --> x", false).context, Context::Slide);
    }

    #[test]
    fn front_matter() {
        assert_eq!(labels("th", "th", true), ["theme"]);
        assert_eq!(analyze("theme: oc", true).context, Context::Value("theme".into()));
        assert_eq!(labels("theme: oc", "theme: oc", true), ["ocean"]);
        assert_eq!(labels("step: fade-", "step: fade-", true).len(), 4);
    }

    #[test]
    fn images() {
        assert_eq!(analyze("![bg c", false).context, Context::Image);
        assert_eq!(labels("![bg c", "![bg c](x.jpg)", false), ["contain", "center"]);
        assert_eq!(analyze("![a](b) c", false).context, Context::Slide);
    }

    #[test]
    fn cheat_sheet_merges_lists() {
        let (_, rows) = cheat_sheet(&Context::Container);
        assert!(rows.iter().any(|(l, _)| l.starts_with("none, fade, fade-up")));
        assert!(rows.iter().any(|(l, _)| l == "1:1, 2:1, 1:2, 1:1:1"));
    }
}
