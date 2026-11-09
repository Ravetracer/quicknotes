#!/usr/bin/env python3
"""Converts third-party colour themes into data/themes.json.

Sources (all MIT licensed, see data/themes/LICENSES.md):
  dracula      https://github.com/dracula/dracula-theme           (palette spec in README.md)
  daylerees    https://github.com/daylerees/colour-schemes        (sublime/*.tmTheme)
  ultimate     https://github.com/CodewithEvilxd/VSCode-Ultimate-Themes-Pack
               (multi-bg-extension/themes/*.json, VS Code themes)

Usage: convert.py <dir with the three repositories cloned> [output.json]

Each theme is reduced to editor colours, a few UI colours and one style per
syntax role. TextMate scopes are resolved like TextMate does: the most
specific matching selector wins, later rules win ties.
"""

import json
import plistlib
import re
import sys
from pathlib import Path

# Syntax roles and the TextMate scopes they are taken from (in order of preference).
ROLES = {
    "comment": ["comment"],
    "string": ["string.quoted", "string"],
    "escape": ["constant.character.escape", "constant.character"],
    "number": ["constant.numeric", "number"],
    "constant": ["constant.language", "constant"],
    "builtin": ["support.function.builtin", "variable.language", "support.constant"],
    "keyword": ["keyword.control", "keyword"],
    "storage": ["storage.modifier", "storage"],
    "type": ["entity.name.type", "storage.type", "support.type", "entity.name.class", "support.class", "type", "class"],
    "function": ["entity.name.function", "support.function", "meta.function-call", "function"],
    "variable": ["variable.other", "variable"],
    "operator": ["keyword.operator"],
    "preprocessor": ["meta.preprocessor", "keyword.control.import", "keyword.control.directive"],
    "tag": ["entity.name.tag"],
    "attribute": ["entity.other.attribute-name"],
    "heading": ["markup.heading", "entity.name.section"],
    "bold": ["markup.bold"],
    "italic": ["markup.italic"],
    "link": ["markup.underline.link", "string.other.link"],
    "inline_code": ["markup.inline.raw", "markup.raw", "markup.fenced_code"],
    "quote": ["markup.quote"],
    "list": ["markup.list", "punctuation.definition.list"],
    "inserted": ["markup.inserted"],
    "deleted": ["markup.deleted"],
    "invalid": ["invalid.illegal", "invalid"],
}

# Roles without a matching rule take the style of another role.
FALLBACK = {
    "escape": "constant",
    "number": "constant",
    "builtin": "constant",
    "storage": "keyword",
    "preprocessor": "keyword",
    "tag": "keyword",
    "attribute": "function",
    "heading": "keyword",
    "link": "string",
    "inline_code": "string",
    "quote": "comment",
    "list": "keyword",
    "inserted": "string",
    "deleted": "invalid",
}


# ----- colours ---------------------------------------------------------------

def parse(c):
    """'#rgb', '#rgba', '#rrggbb', '#rrggbbaa' -> (r, g, b, a) in 0..1, or None."""
    if not isinstance(c, str):
        return None
    c = c.strip()
    if not re.fullmatch(r"#([0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})", c):
        return None
    h = c[1:]
    if len(h) in (3, 4):
        h = "".join(ch * 2 for ch in h)
    if len(h) == 6:
        h += "ff"
    return tuple(int(h[i:i + 2], 16) / 255 for i in range(0, 8, 2))


def hexc(rgb):
    return "#" + "".join(f"{round(max(0, min(1, v)) * 255):02x}" for v in rgb[:3])


def flatten(c, bg):
    """Colour with alpha blended over `bg` -> opaque '#rrggbb'."""
    p = parse(c)
    if p is None:
        return None
    b = parse(bg) or (0, 0, 0, 1)
    a = p[3]
    return hexc([p[i] * a + b[i] * (1 - a) for i in range(3)])


def mix(a, b, t):
    pa, pb = parse(a), parse(b)
    return hexc([pa[i] * (1 - t) + pb[i] * t for i in range(3)])


def luminance(c):
    def lin(v):
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b, _ = parse(c)
    return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)


def contrast(a, b):
    la, lb = sorted([luminance(a), luminance(b)], reverse=True)
    return (la + 0.05) / (lb + 0.05)


# ----- scope resolution --------------------------------------------------------

def selectors(scope):
    """Simple selectors of a rule; descendant/exclusion selectors use their last part."""
    if isinstance(scope, list):
        parts = scope
    else:
        parts = (scope or "").split(",")
    out = []
    for p in parts:
        p = p.strip()
        if not p or "-" in p.split() or p.startswith("-"):
            continue
        out.append(p.split()[-1])
    return out


def matches(selector, scope):
    return scope == selector or scope.startswith(selector + ".")


def resolve(rules, bg, fg):
    """rules: [(selectors, settings)] -> {role: style}."""
    styles = {}
    for role, targets in ROLES.items():
        best = None  # (score, -target index, rule index, settings)
        for ti, target in enumerate(targets):
            for ri, (sels, settings) in enumerate(rules):
                if not settings.get("foreground") and not settings.get("fontStyle"):
                    continue
                for sel in sels:
                    if matches(sel, target):
                        key = (sel.count(".") + 1, -ti, ri)
                        if best is None or key > best[0]:
                            best = (key, settings)
        if best:
            style = style_of(best[1], bg, fg)
            if style:
                styles[role] = style
    for role, other in FALLBACK.items():
        if role not in styles and other in styles:
            styles[role] = dict(styles[other])
    for role in ("bold", "italic"):
        styles.setdefault(role, {})
    styles["bold"]["bold"] = True
    styles["italic"]["italic"] = True
    styles.setdefault("heading", {})["bold"] = True
    styles.setdefault("link", {})["underline"] = True
    return {k: v for k, v in styles.items() if v}


def style_of(settings, bg, fg):
    style = {}
    color = flatten(settings.get("foreground"), bg)
    if color and color != fg:
        style["fg"] = color
    font = (settings.get("fontStyle") or "").lower()
    if "bold" in font:
        style["bold"] = True
    if "italic" in font:
        style["italic"] = True
    if "underline" in font:
        style["underline"] = True
    return style


# ----- theme assembly ----------------------------------------------------------

def assemble(tid, name, source, editor, ui, syntax, dark=None):
    bg, fg = editor["bg"], editor["fg"]
    if dark is None:
        dark = luminance(bg) < 0.18
    shade = "#000000" if dark else "#000000"
    editor.setdefault("line", mix(bg, fg, 0.07))
    editor.setdefault("selection", mix(bg, fg, 0.25))
    editor.setdefault("cursor", fg)
    editor.setdefault("gutter_bg", bg)
    editor.setdefault("gutter_fg", mix(bg, fg, 0.4))
    editor.setdefault("gutter_active", fg)
    # Selections that are hard to read get a softer colour.
    if contrast(editor["selection"], editor.get("selection_fg") or fg) < 2.2:
        editor["selection"] = mix(bg, fg, 0.22)
        editor.pop("selection_fg", None)
    if contrast(editor["gutter_fg"], editor["gutter_bg"]) < 1.6:
        editor["gutter_fg"] = mix(editor["gutter_bg"], fg, 0.4)

    ui = dict(ui)
    ui.setdefault("header", mix(bg, shade, 0.18 if dark else 0.05))
    ui.setdefault("sidebar", ui["header"])
    if not ui.get("accent"):
        for role in ("function", "keyword", "type", "string"):
            c = syntax.get(role, {}).get("fg")
            if c and contrast(c, bg) > 2:
                ui["accent"] = c
                break
        else:
            ui["accent"] = "#3584e4"
    ui.setdefault("link", syntax.get("link", {}).get("fg") or ui["accent"])
    ui.setdefault("muted", syntax.get("comment", {}).get("fg") or mix(bg, fg, 0.5))
    ui["border"] = mix(bg, fg, 0.15)
    # UI backgrounds too close to the text colour (broken values) fall back.
    for key in ("header", "sidebar"):
        if contrast(ui[key], fg) < 3:
            ui[key] = mix(bg, shade, 0.18 if dark else 0.05)
    if editor.get("match") is None:
        editor["match"] = mix(bg, ui["accent"], 0.35)
    return {
        "id": tid,
        "name": name,
        "source": source,
        "dark": bool(dark),
        "editor": editor,
        "ui": ui,
        "syntax": syntax,
    }


def slug(s):
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")


# ----- Dracula ---------------------------------------------------------------

def dracula(repo):
    text = (repo / "README.md").read_text()
    themes = []
    for title, tid, dark in (("Dracula", "dracula", True), ("Alucard", "alucard", False)):
        section = text.split(f"### {title}", 1)[1] if f"### {title}" in text else text
        section = section.split("###", 1)[0]
        pal = {}
        for row in re.finditer(r"^\|\s*([A-Za-z ]+?)\s*\|\s*`(#[0-9a-fA-F]{6})`", section, re.M):
            pal[row.group(1).lower()] = row.group(2).lower()
        if "background" not in pal:
            continue
        p = pal.get
        syntax = {
            "comment": {"fg": p("comment")},
            "string": {"fg": p("yellow")},
            "escape": {"fg": p("pink")},
            "number": {"fg": p("purple")},
            "constant": {"fg": p("purple")},
            "builtin": {"fg": p("cyan")},
            "keyword": {"fg": p("pink")},
            "storage": {"fg": p("pink")},
            "type": {"fg": p("cyan"), "italic": True},
            "function": {"fg": p("green")},
            "operator": {"fg": p("pink")},
            "preprocessor": {"fg": p("pink")},
            "tag": {"fg": p("pink")},
            "attribute": {"fg": p("green"), "italic": True},
            "heading": {"fg": p("purple"), "bold": True},
            "bold": {"fg": p("orange"), "bold": True},
            "italic": {"fg": p("yellow"), "italic": True},
            "link": {"fg": p("cyan"), "underline": True},
            "inline_code": {"fg": p("green")},
            "quote": {"fg": p("yellow"), "italic": True},
            "list": {"fg": p("cyan")},
            "inserted": {"fg": p("green")},
            "deleted": {"fg": p("red")},
            "invalid": {"fg": p("red")},
        }
        bg, fg = p("background"), p("foreground")
        line = p("current line") if dark else mix(bg, fg, 0.06)
        editor = {"bg": bg, "fg": fg, "line": line, "selection": p("selection"),
                  "cursor": fg, "gutter_fg": p("comment")}
        ui = {"accent": p("purple"), "link": p("cyan"), "muted": p("comment")}
        themes.append(assemble(tid, title, "dracula", editor, ui, syntax, dark))
    return themes


# ----- Dayle Rees (tmTheme) -----------------------------------------------------

def tmtheme(path, source):
    data = plistlib.loads(path.read_bytes())
    rules, glob = [], {}
    for item in data.get("settings", []):
        if "scope" not in item:
            if not glob:
                glob = item.get("settings", {})
            continue
        rules.append((selectors(item.get("scope")), item.get("settings", {})))
    bg = flatten(glob.get("background"), "#000000")
    fg = flatten(glob.get("foreground"), bg or "#000000")
    if not bg or not fg:
        return None
    editor = {"bg": bg, "fg": fg}
    for key, src in (("cursor", "caret"), ("line", "lineHighlight"), ("selection", "selection"),
                     ("selection_fg", "selectionForeground"), ("gutter_fg", "gutterForeground"),
                     ("gutter_bg", "gutter"), ("match", "findHighlight")):
        c = flatten(glob.get(src), bg)
        if c:
            editor[key] = c
    syntax = resolve(rules, bg, fg)
    name = data.get("name") or path.stem.replace("-", " ").title()
    return assemble(slug(f"{source}-{name}"), name, source, editor, {}, syntax)


def daylerees(repo):
    files = sorted((repo / "sublime").glob("*.tmTheme")) + sorted((repo / "sublime" / "light").glob("*.tmTheme"))
    out = []
    for f in files:
        t = tmtheme(f, "daylerees")
        if t:
            t["_stem"] = f.stem
            out.append(t)
    return out


# ----- VS Code ---------------------------------------------------------------

def strip_comments(text):
    """Removes // and /* */ comments outside of strings (JSON with comments)."""
    out, i, n, in_str = [], 0, len(text), False
    while i < n:
        c = text[i]
        if in_str:
            out.append(c)
            if c == "\\" and i + 1 < n:
                out.append(text[i + 1])
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
            out.append(c)
        elif text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
            continue
        elif text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = n if end < 0 else end + 2
            continue
        else:
            out.append(c)
        i += 1
    return "".join(out)


def load_jsonc(path):
    text = strip_comments(path.read_text(encoding="utf-8-sig"))
    text = re.sub(r",(\s*[}\]])", r"\1", text)
    # Some files carry trailing garbage after the theme object.
    obj, _ = json.JSONDecoder().raw_decode(text.strip())
    return obj


def vscode(path, source):
    data = load_jsonc(path)
    colors = data.get("colors", {}) or {}
    bg = next((flatten(colors.get(k), "#000000") for k in
               ("editor.background", "editorPane.background", "panel.background", "sideBar.background")
               if colors.get(k)), None)
    if not bg:
        return None
    fg = next((flatten(colors.get(k), bg) for k in
               ("editor.foreground", "foreground", "sideBar.foreground") if colors.get(k)), None)
    if not fg:
        fg = "#d4d4d4" if luminance(bg) < 0.18 else "#1f1f1f"
    rules = []
    for item in data.get("tokenColors", []) or []:
        if isinstance(item, dict) and "scope" in item:
            rules.append((selectors(item.get("scope")), item.get("settings", {}) or {}))
    syntax = resolve(rules, bg, fg)

    def col(*keys, over=None):
        for k in keys:
            c = flatten(colors.get(k), over or bg)
            if c:
                return c
        return None

    editor = {"bg": bg, "fg": fg}
    for key, keys in (("cursor", ["editorCursor.foreground"]),
                      ("line", ["editor.lineHighlightBackground"]),
                      ("selection", ["editor.selectionBackground"]),
                      ("selection_fg", ["editor.selectionForeground"]),
                      ("gutter_bg", ["editorGutter.background"]),
                      ("gutter_fg", ["editorLineNumber.foreground"]),
                      ("gutter_active", ["editorLineNumber.activeForeground"]),
                      ("match", ["editor.findMatchHighlightBackground", "editor.findMatchBackground"])):
        c = col(*keys)
        if c:
            editor[key] = c
    if editor.get("line") == bg:
        editor.pop("line")
    ui = {}
    for key, keys in (("header", ["titleBar.activeBackground", "sideBar.background"]),
                      ("sidebar", ["sideBar.background", "activityBar.background"]),
                      ("accent", ["focusBorder", "button.background", "activityBarBadge.background"]),
                      ("link", ["textLink.foreground"])):
        c = col(*keys)
        if c:
            ui[key] = c
    if ui.get("accent") and contrast(ui["accent"], bg) < 1.8:
        ui.pop("accent")
    kind = data.get("type")
    dark = None if kind not in ("dark", "light", "hc", "hcDark", "hcLight") else kind in ("dark", "hc", "hcDark")
    name = data.get("name") or path.stem.replace("-theme", "").replace("-", " ").title()
    return assemble(slug(f"{source}-{name}"), name, source, editor, ui, syntax, dark)


def ultimate(repo):
    out = []
    for f in sorted((repo / "multi-bg-extension" / "themes").glob("*.json")):
        try:
            t = vscode(f, "ultimate")
        except (ValueError, KeyError) as e:
            print(f"skipped {f.name}: {e}", file=sys.stderr)
            continue
        if t:
            t["_stem"] = f.stem.replace("-theme", "")
            out.append(t)
        else:
            print(f"skipped {f.name}: no editor colours", file=sys.stderr)
    return out


def main():
    src = Path(sys.argv[1])
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parents[2] / "data" / "themes.json"
    themes = dracula(src / "dracula-theme") + daylerees(src / "colour-schemes") + ultimate(src / "VSCode-Ultimate-Themes-Pack")
    # Equal names within a source: use the file names instead.
    names = {}
    for t in themes:
        names.setdefault((t["source"], t["name"]), []).append(t)
    for group in names.values():
        if len(group) > 1:
            for t in group:
                t["name"] = t["_stem"].replace("-", " ").title().replace("Os ", "OS ")
                t["id"] = slug(f"{t['source']}-{t['_stem']}")
    for t in themes:
        t.pop("_stem", None)
    seen = set()
    for t in themes:
        base, n = t["id"], 2
        while t["id"] in seen:
            t["id"] = f"{base}-{n}"
            n += 1
        seen.add(t["id"])
    out.write_text(json.dumps(themes, indent=None, separators=(",", ":"), sort_keys=True) + "\n")
    counts = {}
    for t in themes:
        counts[t["source"]] = counts.get(t["source"], 0) + 1
    print(f"{len(themes)} themes -> {out}: {counts}, dark: {sum(t['dark'] for t in themes)}")


if __name__ == "__main__":
    main()
