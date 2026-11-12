//! Math formulas (LaTeX, rendered with KaTeX) and music notation (ABC,
//! rendered with abcjs) in Markdown documents and presentations.
//!
//! The Markdown output marks formulas as `span.math` (`$…$`, `$$…$$`) or
//! ```` ```math ```` blocks and tunes as ```` ```abc ```` blocks;
//! [`RENDER_JS`] (`qnRender(root)`) typesets them in the page. The libraries
//! are large, so a page only gets the ones its content needs.

const KATEX_JS: &str = include_str!("../data/katex/katex.min.js");
const KATEX_CSS: &str = include_str!("../data/katex/katex.css");
const ABCJS_JS: &str = include_str!("../data/abcjs/abcjs-basic-min.js");

/// Defines `qnRender(root)`, which typesets formulas and tunes below `root`
/// (also in elements not yet attached to the document).
pub const RENDER_JS: &str = r#"
window.qnRender = function (root) {
  if (!root) return;
  if (window.katex) {
    var tex = function (src, el, display) {
      katex.render(src, el, { displayMode: display, throwOnError: false });
    };
    root.querySelectorAll('span.math').forEach(function (el) {
      tex(el.textContent, el, el.classList.contains('math-display'));
    });
    root.querySelectorAll('pre > code.language-math').forEach(function (code) {
      var div = document.createElement('div');
      div.className = 'math math-display';
      tex(code.textContent, div, true);
      code.parentNode.replaceWith(div);
    });
  }
  if (window.ABCJS) {
    // Lay tunes out at the width they get (detached elements: abcjs' default).
    var width = root.isConnected && root.clientWidth > 0 ? Math.max(root.clientWidth - 30, 360) : 740;
    root.querySelectorAll('pre > code.language-abc').forEach(function (code) {
      // abcjs measures text, so it renders into a laid-out (invisible) element.
      var stage = document.createElement('div');
      stage.style.cssText = 'position:absolute;left:-100000px;top:0;visibility:hidden;width:' + (width + 30) + 'px';
      document.body.appendChild(stage);
      // The element keeps the container styles abcjs sets on it.
      var div = document.createElement('div');
      div.className = 'abc';
      stage.appendChild(div);
      try {
        ABCJS.renderAbc(div, code.textContent, {
          responsive: 'resize', add_classes: true, staffwidth: width,
          wrap: { minSpacing: 1.8, maxSpacing: 2.7, preferredMeasuresPerLine: 4 }
        });
      } catch (e) {
        div.className = 'abc error';
        div.textContent = String(e);
      }
      code.parentNode.replaceWith(div);
      stage.remove();
    });
  }
};
"#;

/// Styles for rendered formulas and tunes (besides KaTeX's own).
pub const CSS: &str = r#"
.katex-display { overflow-x: auto; overflow-y: hidden; padding: .2em 0; }
div.math.math-display { margin: 1em 0; }
.abc { margin: 1em 0; }
.abc svg { max-width: 100%; height: auto; }
"#;

/// Notation libraries a page needs.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Libs {
    pub math: bool,
    pub abc: bool,
}

impl Libs {
    /// Libraries needed by rendered HTML (also works on JSON with that HTML).
    pub fn needed(html: &str) -> Self {
        Self {
            math: html.contains("math math-") || html.contains("language-math"),
            abc: html.contains("language-abc"),
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self { math: self.math || other.math, abc: self.abc || other.abc }
    }

    /// Libraries of `self` that are not in `loaded`.
    pub fn missing(self, loaded: Self) -> Self {
        Self { math: self.math && !loaded.math, abc: self.abc && !loaded.abc }
    }

    /// `<style>`/`<script>` tags for a page's `<head>`.
    pub fn head(self) -> String {
        let mut out = format!("<style>{CSS}</style>\n<script>{RENDER_JS}</script>\n");
        if self.math {
            out.push_str(&format!("<style>{KATEX_CSS}</style>\n<script>{KATEX_JS}</script>\n"));
        }
        if self.abc {
            out.push_str(&format!("<script>{ABCJS_JS}</script>\n"));
        }
        out
    }

    /// JavaScript that loads the libraries into a running page (which
    /// already has [`RENDER_JS`] and [`CSS`]). Empty if there are none.
    pub fn inject_js(self) -> String {
        let mut out = String::new();
        if self.math {
            let css = serde_json::to_string(KATEX_CSS).unwrap_or_default();
            out.push_str(&format!(
                "(function(){{var s=document.createElement('style');s.textContent={css};document.head.appendChild(s);}})();\n"
            ));
            out.push_str(KATEX_JS);
            out.push_str("\n;\n");
        }
        if self.abc {
            out.push_str(ABCJS_JS);
            out.push_str("\n;\n");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_needed_libraries() {
        let html = crate::markdown::to_html("Euler: $e^{i\\pi} + 1 = 0$\n\n```abc\nX:1\nK:C\nCDEF|\n```\n");
        assert_eq!(Libs::needed(&html), Libs { math: true, abc: true });
        assert_eq!(Libs::needed(&crate::markdown::to_html("```math\nx^2\n```\n")), Libs { math: true, abc: false });
        assert_eq!(Libs::needed(&crate::markdown::to_html("costs $5 and $10\n")), Libs::default());
    }

    #[test]
    fn injects_only_missing_libraries() {
        let need = Libs { math: true, abc: true };
        assert_eq!(need.missing(Libs { math: true, abc: false }), Libs { math: false, abc: true });
        assert!(Libs::default().inject_js().is_empty());
        assert!(Libs { math: false, abc: true }.inject_js().contains("ABCJS"));
    }

    #[test]
    fn libraries_are_safe_inline() {
        for src in [KATEX_JS, KATEX_CSS, ABCJS_JS] {
            assert!(!src.to_lowercase().contains("</script") && !src.to_lowercase().contains("</style"));
        }
    }
}
