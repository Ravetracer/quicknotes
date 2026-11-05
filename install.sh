#!/usr/bin/env bash
# Installs QuickNotes for the current user (no root needed):
#   binary  -> ~/.local/bin/quicknotes
#   desktop -> ~/.local/share/applications/io.github.quicknotes.desktop
#   icons   -> ~/.local/share/icons/hicolor/<size>/apps/io.github.quicknotes.png
# Builds the release binary in the dev container first if it is missing.
set -euo pipefail
cd "$(dirname "$0")"

BIN=target/release/quicknotes
[[ -x "$BIN" ]] || ./dev.sh build --release

PREFIX="$HOME/.local"
install -Dm755 "$BIN" "$PREFIX/bin/quicknotes"
for icon in data/icons/hicolor/*/apps/io.github.quicknotes.png; do
    install -Dm644 "$icon" "$PREFIX/share/${icon#data/}"
done
# Placeholder icon of earlier versions
rm -f "$PREFIX/share/icons/hicolor/scalable/apps/io.github.quicknotes.svg"

mkdir -p "$PREFIX/share/applications"
cat > "$PREFIX/share/applications/io.github.quicknotes.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=QuickNotes
GenericName=Notepad
Comment=Tabbed notepad for text and code snippets
Exec=$PREFIX/bin/quicknotes %F
Icon=io.github.quicknotes
Terminal=false
Categories=Utility;TextEditor;Development;
MimeType=text/plain;text/markdown;application/json;application/xml;text/xml;application/x-yaml;text/x-log;
Keywords=notes;editor;snippet;json;markdown;plantuml;
StartupNotify=true
StartupWMClass=quicknotes
EOF

update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
echo "Installed QuickNotes to $PREFIX/bin/quicknotes"
