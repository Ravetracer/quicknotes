#!/usr/bin/env bash
# Runs cargo inside the development container. The host only needs Docker or
# Podman.
#   ./dev.sh                  -> cargo build --release
#   ./dev.sh test             -> cargo test
#   ./dev.sh clippy           -> cargo clippy
#   ./dev.sh rebuild-image    -> rebuild the dev image
#   ./dev.sh appimage         -> release build + target/QuickNotes-<version>-x86_64.AppImage
#
# Container engine: Docker if installed, otherwise Podman. Override with a
# leading --docker / --podman or CONTAINER_ENGINE=docker|podman.
set -euo pipefail
cd "$(dirname "$0")"
IMAGE=quicknotes-dev

case "${1:-}" in
    --docker|--podman) ENGINE=${1#--}; shift ;;
    *) ENGINE=${CONTAINER_ENGINE:-} ;;
esac
if [[ -z "$ENGINE" ]]; then
    if command -v docker >/dev/null; then ENGINE=docker
    elif command -v podman >/dev/null; then ENGINE=podman
    else echo "Neither docker nor podman found" >&2; exit 1
    fi
fi
command -v "$ENGINE" >/dev/null || { echo "$ENGINE not found" >&2; exit 1; }

if [[ "$ENGINE" == podman ]]; then
    # Rootless Podman: keep the host UID so build output belongs to the user;
    # no SELinux relabeling of the project directory (Fedora, Bazzite).
    USER_OPTS=(--userns=keep-id --security-opt label=disable)
else
    USER_OPTS=(-u "$(id -u):$(id -g)")
fi

if [[ "${1:-}" == "rebuild-image" ]]; then
    "$ENGINE" build -t "$IMAGE" docker/
    exit 0
fi
if ! "$ENGINE" image inspect "$IMAGE" >/dev/null 2>&1; then
    "$ENGINE" build -t "$IMAGE" docker/
fi

run() {
    mkdir -p .docker-cargo
    "$ENGINE" run --rm -t \
        "${USER_OPTS[@]}" \
        -v "$PWD:/src" \
        -e CARGO_HOME=/src/.docker-cargo \
        -w /src \
        "$IMAGE" "$@"
}

if [[ "${1:-}" == "appimage" ]]; then
    run cargo build --release
    run packaging/appimage/build-appimage.sh
    exit 0
fi

args=("$@")
[[ ${#args[@]} -eq 0 ]] && args=(build --release)
run cargo "${args[@]}"
