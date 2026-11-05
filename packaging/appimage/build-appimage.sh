#!/usr/bin/env bash
# Builds target/QuickNotes-<version>-x86_64.AppImage. Runs inside the dev
# container (see ./dev.sh appimage) after `cargo build --release`.
set -euo pipefail
cd "$(dirname "$0")/../.."

PLANTUML_VERSION=1.2026.9
APPIMAGETOOL_VERSION=1.9.1
RUNTIME_VERSION=20251108

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
SYS=/usr/lib/x86_64-linux-gnu
PKG=packaging/appimage
CACHE=target/appimage-cache
APPDIR=target/appimage/QuickNotes.AppDir
OUT=target/QuickNotes-$VERSION-x86_64.AppImage
APP_ID=io.github.quicknotes

download() { # url target
    [[ -s "$2" ]] || { echo "Downloading $(basename "$2")"; curl -fsSL -o "$2.part" "$1" && mv "$2.part" "$2"; }
}

mkdir -p "$CACHE"
download "https://github.com/plantuml/plantuml/releases/download/v$PLANTUML_VERSION/plantuml-mit-$PLANTUML_VERSION.jar" \
    "$CACHE/plantuml-mit-$PLANTUML_VERSION.jar"
download "https://github.com/AppImage/appimagetool/releases/download/$APPIMAGETOOL_VERSION/appimagetool-x86_64.AppImage" \
    "$CACHE/appimagetool-$APPIMAGETOOL_VERSION.AppImage"
download "https://github.com/AppImage/type2-runtime/releases/download/$RUNTIME_VERSION/runtime-x86_64" \
    "$CACHE/runtime-$RUNTIME_VERSION"
# appimagetool is itself an AppImage; there is no FUSE in the container.
if [[ ! -x "$CACHE/appimagetool-$APPIMAGETOOL_VERSION/AppRun" ]]; then
    chmod +x "$CACHE/appimagetool-$APPIMAGETOOL_VERSION.AppImage"
    (cd "$CACHE" && "./appimagetool-$APPIMAGETOOL_VERSION.AppImage" --appimage-extract >/dev/null \
        && rm -rf "appimagetool-$APPIMAGETOOL_VERSION" && mv squashfs-root "appimagetool-$APPIMAGETOOL_VERSION")
fi

echo "Assembling AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/lib" "$APPDIR/usr/share"
U="$APPDIR/usr"

# --- QuickNotes, start scripts, desktop entry -------------------------------
install -m755 target/release/quicknotes "$U/bin/quicknotes"
install -m755 "$PKG/AppRun" "$APPDIR/AppRun"
install -m755 "$PKG/plantuml" "$U/bin/plantuml"
cat > "$APPDIR/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=QuickNotes
GenericName=Notepad
Comment=Tabbed notepad for text and code snippets
Exec=quicknotes %F
Icon=$APP_ID
Terminal=false
Categories=Utility;TextEditor;Development;
MimeType=text/plain;text/markdown;application/json;application/xml;text/xml;application/x-yaml;
StartupWMClass=quicknotes
EOF
cp data/icons/hicolor/256x256/apps/$APP_ID.png "$APPDIR/$APP_ID.png"
ln -s "$APP_ID.png" "$APPDIR/.DirIcon"

# --- WebKitGTK helper processes ----------------------------------------------
# WebKit starts them from a path compiled into libwebkitgtk; it is patched
# below to a relative path, which AppRun resolves by changing into usr/.
mkdir -p "$U/lib/x86_64-linux-gnu/webkitgtk-6.0"
cp -r "$SYS/webkitgtk-6.0/"{WebKitWebProcess,WebKitNetworkProcess,WebKitGPUProcess,injected-bundle} \
    "$U/lib/x86_64-linux-gnu/webkitgtk-6.0/"

# --- GTK modules and data ----------------------------------------------------
mkdir -p "$U/lib/gtk-4.0/4.0.0/printbackends" "$U/lib/gio/modules" "$U/lib/gdk-pixbuf-2.0/loaders"
cp "$SYS/gtk-4.0/4.0.0/printbackends/libprintbackend-file.so" "$U/lib/gtk-4.0/4.0.0/printbackends/"
cp "$SYS/gio/modules/libgiognutls.so" "$U/lib/gio/modules/"
cp "$SYS/gdk-pixbuf-2.0/2.10.0/loaders/"*.so "$U/lib/gdk-pixbuf-2.0/loaders/"
mkdir -p "$U/share/glib-2.0/schemas"
cp /usr/share/glib-2.0/schemas/org.gtk.gtk4.*.gschema.xml "$U/share/glib-2.0/schemas/"
glib-compile-schemas "$U/share/glib-2.0/schemas"
cp -r /usr/share/gtksourceview-5 "$U/share/"
# File type database (compiled files only) for GdkPixbuf and GIO.
mkdir -p "$U/share/mime"
find /usr/share/mime -maxdepth 1 -type f -exec cp {} "$U/share/mime/" \;
mkdir -p "$U/share/icons"
cp -r /usr/share/icons/Adwaita "$U/share/icons/"
mkdir -p "$U/share/icons/hicolor"
cp /usr/share/icons/hicolor/index.theme "$U/share/icons/hicolor/"
cp -r data/icons/hicolor/* "$U/share/icons/hicolor/"

# --- PlantUML: Java runtime, jar, Graphviz -----------------------------------
cp -rL /usr/lib/jvm/java-21-openjdk-amd64 "$U/lib/jvm"
rm -rf "$U/lib/jvm/man" "$U/lib/jvm/docs"
mkdir -p "$U/share/plantuml"
cp "$CACHE/plantuml-mit-$PLANTUML_VERSION.jar" "$U/share/plantuml/plantuml.jar"
cp /usr/bin/dot "$U/bin/dot"
mkdir -p "$U/lib/graphviz"
for plugin in core dot_layout neato_layout pango; do
    cp "$SYS/graphviz/libgvplugin_$plugin.so.6" "$U/lib/graphviz/"
done

# --- shared libraries ---------------------------------------------------------
echo "Collecting libraries"
mapfile -t elf_files < <(
    printf '%s\n' "$U/bin/quicknotes" "$U/bin/dot"
    find "$U/lib/x86_64-linux-gnu" "$U/lib/gtk-4.0" "$U/lib/gio" "$U/lib/gdk-pixbuf-2.0" \
        "$U/lib/graphviz" "$U/lib/jvm" -type f \( -name '*.so*' -o -perm -u+x \) |
        while read -r f; do file -b "$f" | grep -q '^ELF' && echo "$f"; done
)
excludes=$(grep -vE '^\s*(#|$)' "$PKG/excludelist" | paste -sd'|')
ldd "${elf_files[@]}" 2>/dev/null | awk '/=> \//{print $3}' | sort -u | while read -r lib; do
    name=$(basename "$lib")
    case "$lib" in /usr/lib/jvm/*) continue ;; esac # Java brings its own
    if echo "$name" | grep -qE "$excludes"; then continue; fi
    cp -L "$lib" "$U/lib/$name"
done
# Check: everything resolves, except excluded (system) libraries and libjvm,
# which Java loads from its own directory.
missing=$(LD_LIBRARY_PATH="$U/lib" ldd "${elf_files[@]}" 2>/dev/null | grep 'not found' |
    awk '{print $1}' | grep -vE "$excludes|^libjvm\.so" | sort -u || true)
[[ -z "$missing" ]] || { echo "Unresolved libraries:"; echo "$missing"; exit 1; }

# Relative WebKit paths (same length as the original, see AppRun).
# "/usr" -> "././" keeps the length, so the binary layout stays intact.
perl -pi -e 's|/usr(/lib/x86_64-linux-gnu/webkitgtk-6\.0)|././$1|g' "$U/lib/libwebkitgtk-6.0.so.4"
grep -qa '././/lib/x86_64-linux-gnu/webkitgtk-6.0/injected-bundle/' "$U/lib/libwebkitgtk-6.0.so.4"
[[ $(stat -c %s "$U/lib/libwebkitgtk-6.0.so.4") == $(stat -L -c %s "$SYS/libwebkitgtk-6.0.so.4") ]] ||
    { echo "Patching libwebkitgtk changed its size"; exit 1; }

# --- caches that need the final layout ----------------------------------------
GDK_PIXBUF_MODULEDIR="$U/lib/gdk-pixbuf-2.0/loaders" LD_LIBRARY_PATH="$U/lib" \
    "$SYS/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders" |
    sed "s|$(readlink -f "$APPDIR")|@APPDIR@|g; s|$APPDIR|@APPDIR@|g" > "$U/lib/gdk-pixbuf-2.0/loaders.cache.in"
grep -q '@APPDIR@' "$U/lib/gdk-pixbuf-2.0/loaders.cache.in"
# Graphviz plugin list: generated with absolute paths, stored with bare file
# names (resolved against GVBINDIR at runtime, see the plantuml wrapper).
GV=$(readlink -f "$U/lib/graphviz")
GVBINDIR="$GV" LD_LIBRARY_PATH="$U/lib" "$U/bin/dot" -c
sed -i "s|$GV/||g" "$GV/config6a"
grep -q '^libgvplugin_dot_layout.so.6 ' "$GV/config6a"

# --- AppImage ----------------------------------------------------------------
echo "Building $OUT"
rm -f "$OUT"
ARCH=x86_64 "$CACHE/appimagetool-$APPIMAGETOOL_VERSION/AppRun" --no-appstream \
    --runtime-file "$CACHE/runtime-$RUNTIME_VERSION" "$APPDIR" "$OUT" >/dev/null
du -h "$OUT"
