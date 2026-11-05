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

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustup component add clippy rustfmt \
    && chmod -R a+rX /usr/local/rustup /usr/local/cargo

WORKDIR /src
