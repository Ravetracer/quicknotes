//! Emmet abbreviations (<https://emmet.io>) for HTML, XML/XSLT, CSS, SCSS and
//! Less, using the official Emmet engine (`data/emmet`, MIT) in
//! JavaScriptCore. Expansions are returned in GtkSourceView snippet syntax,
//! so the fields of an expansion become tab stops.

use std::cell::RefCell;
use webkit6::javascriptcore as jsc;

const BUNDLE: &str = include_str!("../data/emmet/emmet.cjs");

/// Wraps the CommonJS bundle and exposes a small JSON API as `QN`.
const GLUE: &str = r#"
var QN = (function (emmet) {
  // Literal text must not be read as snippet syntax.
  function esc(s) { return String(s).replace(/[\\$}]/g, '\\$&'); }
  function config(a) {
    var c = {
      type: a.type,
      syntax: a.syntax,
      options: {
        'output.field': function (index, placeholder) {
          return '${' + index + (placeholder ? ':' + esc(placeholder) : '') + '}';
        },
        'output.text': esc,
        'output.indent': a.indent,
        'output.baseIndent': a.baseIndent || ''
      }
    };
    if (a.text != null) { c.text = a.text; }
    return c;
  }
  var known = null;
  return {
    expand: function (a) { return emmet.default(a.abbr, config(a)); },
    extract: function (a) {
      var r = emmet.extract(a.line, a.line.length, { type: a.type, lookAhead: false });
      return r ? { abbr: r.abbreviation, start: r.start } : null;
    },
    // HTML element and snippet names; other single words are no abbreviations.
    isKnown: function (word) {
      if (!known) {
        known = {};
        var snippets = emmet.resolveConfig({ type: 'markup', syntax: 'html' }).snippets;
        Object.keys(snippets).forEach(function (k) {
          k.split('|').forEach(function (n) { known[n.toLowerCase()] = true; });
        });
        ('a abbr address area article aside audio b base bdi bdo blockquote body br button canvas caption ' +
         'cite code col colgroup data datalist dd del details dfn dialog div dl dt em embed fieldset ' +
         'figcaption figure footer form h1 h2 h3 h4 h5 h6 head header hgroup hr html i iframe img input ins ' +
         'kbd label legend li link main map mark menu meta meter nav noscript object ol optgroup option ' +
         'output p param picture pre progress q rp rt ruby s samp script search section select slot small ' +
         'source span strong style sub summary sup table tbody td template textarea tfoot th thead time ' +
         'title tr track u ul var video wbr svg').split(' ').forEach(function (n) { known[n] = true; });
      }
      return !!known[word.toLowerCase()] || /^(lorem|lipsum)\d*$/i.test(word);
    }
  };
})(module.exports);
"#;

/// Emmet syntax of a GtkSourceView language.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Syntax {
    pub name: &'static str,
    pub stylesheet: bool,
}

const CSS: Syntax = Syntax { name: "css", stylesheet: true };

/// Emmet is only available for these languages.
pub fn syntax_for(language: Option<&str>) -> Option<Syntax> {
    let (name, stylesheet) = match language? {
        "html" => ("html", false),
        "xml" => ("xml", false),
        "xslt" => ("xsl", false),
        "css" => ("css", true),
        "scss" => ("scss", true),
        "less" => ("less", true),
        _ => return None,
    };
    Some(Syntax { name, stylesheet })
}

thread_local! {
    static CONTEXT: RefCell<Option<jsc::Context>> = const { RefCell::new(None) };
}

/// Calls `QN.<function>(args)` and returns the result as JSON.
fn call(function: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    CONTEXT.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            let ctx = jsc::Context::new();
            ctx.evaluate("var module = { exports: {} }; var exports = module.exports;");
            ctx.evaluate(BUNDLE);
            ctx.evaluate(GLUE);
            if let Some(e) = ctx.exception() {
                return Err(format!("Emmet: {}", e.to_str()));
            }
            *slot = Some(ctx);
        }
        let ctx = slot.as_ref().expect("context");
        ctx.clear_exception();
        let code = format!("JSON.stringify(QN.{function}({args}))");
        let value = ctx.evaluate(&code);
        if let Some(e) = ctx.exception() {
            ctx.clear_exception();
            return Err(e.message().map(|m| m.to_string()).unwrap_or_else(|| e.to_str().to_string()));
        }
        let text = value.map(|v| v.to_str().to_string()).unwrap_or_default();
        serde_json::from_str(&text).map_err(|e| e.to_string())
    })
}

/// An abbreviation found before the cursor.
#[derive(Debug, PartialEq, Eq)]
pub struct Found {
    pub abbreviation: String,
    /// Start in characters, relative to the text passed to `extract`.
    pub start: usize,
}

/// Finds the abbreviation that ends at the end of `before` (the line up to
/// the cursor).
pub fn extract(before: &str, syntax: Syntax) -> Option<Found> {
    let r = call("extract", &serde_json::json!({ "line": before, "type": kind(syntax) })).ok()?;
    let abbreviation = r.get("abbr")?.as_str()?.to_owned();
    let start_utf16 = r.get("start")?.as_u64()? as usize;
    // JavaScript counts UTF-16 units.
    let mut units = 0;
    let start = before.chars().take_while(|c| {
        let take = units < start_utf16;
        units += c.len_utf16();
        take
    });
    let start = start.count();
    (!abbreviation.trim().is_empty()).then_some(Found { abbreviation, start })
}

/// Whether a single word is an HTML element or snippet name.
pub fn is_known_word(word: &str) -> bool {
    call("isKnown", &serde_json::json!(word)).ok().and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Expands `abbreviation` into GtkSourceView snippet syntax. `wrap` is
/// wrapped by the abbreviation (one entry per line).
pub fn expand(
    abbreviation: &str,
    syntax: Syntax,
    indent: &str,
    base_indent: &str,
    wrap: Option<&[&str]>,
) -> Result<String, String> {
    let args = serde_json::json!({
        "abbr": abbreviation,
        "type": kind(syntax),
        "syntax": syntax.name,
        "indent": indent,
        "baseIndent": base_indent,
        "text": wrap,
    });
    let out = call("expand", &args)?;
    out.as_str().map(str::to_owned).ok_or_else(|| "no result".into())
}

fn kind(syntax: Syntax) -> &'static str {
    if syntax.stylesheet {
        "stylesheet"
    } else {
        "markup"
    }
}

/// The syntax that applies at the end of `before` (text of the document up
/// to the cursor): CSS inside an HTML `<style>` element, nothing inside
/// `<script>` or within a tag, otherwise the document's syntax.
pub fn syntax_at(before: &str, syntax: Syntax) -> Option<Syntax> {
    if syntax.stylesheet {
        return Some(syntax);
    }
    let lower = before.to_ascii_lowercase();
    if syntax.name == "html" {
        let open = |tag: &str| lower.rfind(&format!("<{tag}"));
        let close = |tag: &str| lower.rfind(&format!("</{tag}"));
        let inside = |tag: &str| matches!((open(tag), close(tag)), (Some(o), c) if c.is_none_or(|c| c < o));
        if inside("script") {
            return None;
        }
        if inside("style") {
            // Still within the `<style …>` tag itself: attributes.
            let start = open("style").unwrap_or(0);
            lower[start..].find('>')?;
            return Some(CSS);
        }
    }
    // Within a tag (between `<` and `>`): attributes, no abbreviations.
    match (lower.rfind('<'), lower.rfind('>')) {
        (Some(lt), gt) if gt.is_none_or(|gt| gt < lt) => None,
        _ => Some(syntax),
    }
}

/// Whether Tab should expand `found` (stricter than an explicit expansion).
pub fn tab_expands(found: &Found, syntax: Syntax, before: &str) -> bool {
    let word = !found.abbreviation.is_empty()
        && found.abbreviation.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if syntax.stylesheet {
        // Only property abbreviations inside a rule block.
        let depth = before.matches('{').count() as i64 - before.matches('}').count() as i64;
        return depth > 0;
    }
    // Plain words in HTML only if they are elements or snippets.
    !(word && syntax.name == "html" && !is_known_word(&found.abbreviation))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML: Syntax = Syntax { name: "html", stylesheet: false };

    #[test]
    fn expands_markup_with_tab_stops() {
        let out = expand("ul>li.item$*2", HTML, "  ", "", None).unwrap();
        assert_eq!(out, "<ul>\n  <li class=\"item1\">${1}</li>\n  <li class=\"item2\">${2}</li>\n</ul>");
        let out = expand("a[href]", HTML, "  ", "", None).unwrap();
        assert_eq!(out, "<a href=\"${1}\">${2}</a>");
    }

    #[test]
    fn escapes_snippet_syntax_in_text() {
        // `\$` is a literal dollar in Emmet (`$` alone is the item number).
        let out = expand("p{costs \\$5 {x}}", HTML, "  ", "", None).unwrap();
        assert_eq!(out, "<p>costs \\$5 {x\\}</p>");
    }

    #[test]
    fn expands_stylesheets() {
        assert_eq!(expand("m10", CSS, "  ", "", None).unwrap(), "margin: 10px;");
        assert_eq!(expand("df", CSS, "  ", "", None).unwrap(), "display: flex;");
    }

    #[test]
    fn wraps_text() {
        let out = expand("ul>li*", HTML, "  ", "", Some(&["one", "two"])).unwrap();
        assert_eq!(out, "<ul>\n  <li>one</li>\n  <li>two</li>\n</ul>");
    }

    #[test]
    fn extracts_before_cursor() {
        let found = extract("  text ül div>p", HTML).unwrap();
        assert_eq!(found, Found { abbreviation: "div>p".into(), start: 10 });
        assert!(extract("   ", HTML).is_none());
    }

    #[test]
    fn supported_languages_only() {
        assert!(syntax_for(Some("html")).is_some());
        assert!(syntax_for(Some("scss")).is_some_and(|s| s.stylesheet));
        assert!(syntax_for(Some("markdown")).is_none());
        assert!(syntax_for(Some("php")).is_none());
        assert!(syntax_for(None).is_none());
    }

    #[test]
    fn context_rules() {
        assert_eq!(syntax_at("<style>\nbody {\n  m10", HTML), Some(CSS));
        assert_eq!(syntax_at("<style>a{}</style>\n<p>", HTML), Some(HTML));
        assert_eq!(syntax_at("<script>\nlet x", HTML), None);
        assert_eq!(syntax_at("<div title", HTML), None);
        let found = |a: &str| Found { abbreviation: a.into(), start: 0 };
        assert!(tab_expands(&found("div"), HTML, "div"));
        assert!(!tab_expands(&found("hello"), HTML, "hello"));
        assert!(tab_expands(&found("hello>world"), HTML, "hello>world"));
        assert!(!tab_expands(&found("m10"), CSS, "body m10"));
        assert!(tab_expands(&found("m10"), CSS, "body {\n  m10"));
    }
}
