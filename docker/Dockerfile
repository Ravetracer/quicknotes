# Development / build image for QuickNotes.
# Matches the host distribution (Ubuntu 24.04) so the resulting binary links
# against exactly the GTK4 / libadwaita / GtkSourceView / WebKitGTK runtime
# libraries that are already installed on the desktop.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl build-essential pkg-config git \
        libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev libwebkitgtk-6.0-dev \
    && rm -rf /var/lib/apt/lists/*

# Everything bundled into the AppImage (./dev.sh appimage): Java and Graphviz
# for PlantUML, icons, image loaders, TLS for WebKit, packaging tools.
RUN apt-get update && apt-get install -y --no-install-recommends \
        openjdk-21-jre-headless graphviz \
        adwaita-icon-theme librsvg2-common webp-pixbuf-loader libgdk-pixbuf2.0-bin \
        glib-networking libglib2.0-bin \
        file desktop-file-utils perl \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustup component add clippy rustfmt \
    && chmod -R a+rX /usr/local/rustup /usr/local/cargo

WORKDIR /src

# Secret Service (GNOME keyring) for the Google Drive token. Separate layer so
# adding it does not invalidate the toolchain layers above.
RUN apt-get update && apt-get install -y --no-install-recommends libsecret-1-dev \
    && rm -rf /var/lib/apt/lists/*
