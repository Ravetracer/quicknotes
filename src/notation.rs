//! Math formulas (LaTeX, rendered with KaTeX), music notation (ABC,
//! rendered with abcjs) and charts (Apache ECharts) in Markdown documents and
//! presentations.
//!
//! The Markdown output marks formulas as `span.math` (`$…$`, `$$…$$`) or
//! ```` ```math ```` blocks, tunes as ```` ```abc ```` blocks and charts as
//! ```` ```qn-chart ```` blocks (see [`crate::charts`]); [`RENDER_JS`]
//! (`qnRender(root)`) typesets them in the page. The libraries are large, so
//! a page only gets the ones its content needs.

const KATEX_JS: &str = include_str!("../data/katex/katex.min.js");
const KATEX_CSS: &str = include_str!("../data/katex/katex.css");
const ABCJS_JS: &str = include_str!("../data/abcjs/abcjs-basic-min.js");
const ECHARTS_JS: &str = include_str!("../data/echarts/echarts.min.js");

/// Defines `qnRender(root)`, which typesets formulas, tunes and charts below
/// `root` (also in elements not yet attached to the document: their charts
/// are drawn once they are laid out, or by `qnDraw(root)`), and
/// `qnRetheme()`, which redraws charts after a theme change.
pub const RENDER_JS: &str = r#"
(function () {
  var drawn = [];
  var observer = null;
  function darkText(el) {
    var m = (getComputedStyle(el).color || '').match(/[\d.]+/g);
    return !!m && (0.299 * m[0] + 0.587 * m[1] + 0.114 * m[2]) > 140;
  }
  function draw(div) {
    var spec = div.qnSpec;
    var dark = spec.theme === 'dark' || (spec.theme !== 'light' && darkText(div));
    // Not laid out (e.g. an off-screen print view): a typical page width.
    var size = div.clientWidth > 0 ? null : { width: 680 };
    var chart = echarts.init(div, dark ? 'dark' : null, { renderer: 'svg', width: size && size.width });
    // Scalable when printed at another width.
    chart.on('finished', function () {
      var svg = div.querySelector('svg');
      if (svg && svg.getAttribute('width')) svg.setAttribute('viewBox', '0 0 ' + parseFloat(svg.getAttribute('width')) + ' ' + parseFloat(svg.getAttribute('height')));
    });
    try {
      chart.setOption(spec.option);
      if (drawn.indexOf(div) < 0) drawn.push(div);
    } catch (e) {
      chart.dispose();
      if (observer) observer.unobserve(div);
      div.qnSpec = null;
      div.className = 'qn-chart error';
      div.style.height = '';
      div.textContent = String((e && e.message) || e);
    }
  }
  function watch(div) {
    if (!window.ResizeObserver) return;
    if (!observer) observer = new ResizeObserver(function (entries) {
      entries.forEach(function (e) {
        var div = e.target;
        if (!div.qnSpec || !div.isConnected || div.clientWidth === 0) return;
        var chart = echarts.getInstanceByDom(div);
        if (chart) chart.resize(); else draw(div);
      });
    });
    observer.observe(div);
  }
  /** Releases charts that are no longer in the document. */
  function sweep() {
    drawn = drawn.filter(function (div) {
      if (div.isConnected) return true;
      var chart = echarts.getInstanceByDom(div);
      if (chart) chart.dispose();
      if (observer) observer.unobserve(div);
      return false;
    });
  }
  window.qnCharts = function (root) {
    sweep();
    root.querySelectorAll('pre > code.language-qn-chart').forEach(function (code) {
      var spec;
      try { spec = JSON.parse(code.textContent); } catch (e) { spec = { error: String(e) }; }
      var div = document.createElement('div');
      if (spec.error) {
        div.className = 'qn-chart error';
        div.textContent = spec.error;
      } else {
        div.className = 'qn-chart';
        div.style.height = spec.height || '360px';
        div.qnSpec = spec;
        watch(div);
      }
      code.parentNode.replaceWith(div);
    });
    window.qnDraw(root);
  };
  /** Draws the pending charts below `root` once it is in the document. */
  window.qnDraw = function (root) {
    if (!window.echarts || !root || !root.isConnected) return;
    root.querySelectorAll('div.qn-chart').forEach(function (div) {
      if (div.qnSpec && !echarts.getInstanceByDom(div)) draw(div);
    });
  };
  window.qnRetheme = function () {
    if (!window.echarts) return;
    drawn.slice().forEach(function (div) {
      var chart = echarts.getInstanceByDom(div);
      if (chart && div.qnSpec && div.qnSpec.theme === 'auto') {
        chart.dispose();
        draw(div);
      }
    });
  };
})();
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
  if (window.echarts) qnCharts(root);
};
"#;

/// Styles for rendered formulas and tunes (besides KaTeX's own).
pub const CSS: &str = r#"
.katex-display { overflow-x: auto; overflow-y: hidden; padding: .2em 0; }
div.math.math-display { margin: 1em 0; }
.abc { margin: 1em 0; }
.abc svg { max-width: 100%; height: auto; }
.qn-chart { width: 100%; margin: 1em 0; break-inside: avoid; box-sizing: border-box; }
.qn-chart.error {
  color: var(--error, #c01c28); border: 1px solid currentColor; border-radius: 6px; padding: .5em .8em;
  font-family: "Monospace", monospace; font-size: .85em; white-space: pre-wrap;
}
@media print {
  .qn-chart:not(.error) { height: auto !important; }
  .qn-chart > div { width: 100% !important; height: auto !important; }
  .qn-chart svg { position: static !important; display: block; width: 100% !important; height: auto !important; }
}
"#;

/// Notation libraries a page needs.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Libs {
    pub math: bool,
    pub abc: bool,
    pub charts: bool,
}

impl Libs {
    /// Libraries needed by rendered HTML (also works on JSON with that HTML).
    pub fn needed(html: &str) -> Self {
        Self {
            math: html.contains("math math-") || html.contains("language-math"),
            abc: html.contains("language-abc"),
            charts: html.contains("language-qn-chart"),
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self { math: self.math || other.math, abc: self.abc || other.abc, charts: self.charts || other.charts }
    }

    /// Libraries of `self` that are not in `loaded`.
    pub fn missing(self, loaded: Self) -> Self {
        Self { math: self.math && !loaded.math, abc: self.abc && !loaded.abc, charts: self.charts && !loaded.charts }
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
        if self.charts {
            out.push_str(&format!("<script>{ECHARTS_JS}</script>\n"));
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
        if self.charts {
            out.push_str(ECHARTS_JS);
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
        assert_eq!(Libs::needed(&html), Libs { math: true, abc: true, charts: false });
        assert_eq!(Libs::needed(&crate::markdown::to_html("```math\nx^2\n```\n")), Libs { math: true, ..Default::default() });
        let chart = crate::markdown::to_html("| a | b |\n|---|---|\n| x | 1 |\n\n```chart\ntype: bar\n```\n");
        assert_eq!(Libs::needed(&chart), Libs { charts: true, ..Default::default() });
        assert_eq!(Libs::needed(&crate::markdown::to_html("costs $5 and $10\n")), Libs::default());
    }

    #[test]
    fn injects_only_missing_libraries() {
        let need = Libs { math: true, abc: true, charts: true };
        let loaded = Libs { math: true, ..Default::default() };
        assert_eq!(need.missing(loaded), Libs { math: false, abc: true, charts: true });
        assert!(Libs::default().inject_js().is_empty());
        assert!(Libs { abc: true, ..Default::default() }.inject_js().contains("ABCJS"));
        assert!(Libs { charts: true, ..Default::default() }.inject_js().contains("echarts"));
    }

    #[test]
    fn libraries_are_safe_inline() {
        for src in [KATEX_JS, KATEX_CSS, ABCJS_JS, ECHARTS_JS, RENDER_JS] {
            assert!(!src.to_lowercase().contains("</script") && !src.to_lowercase().contains("</style"));
        }
    }
}
