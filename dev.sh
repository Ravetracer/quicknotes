#!/usr/bin/env bash
# Runs cargo inside the development container. The host only needs Docker.
#   ./dev.sh                  -> cargo build --release
#   ./dev.sh test             -> cargo test
#   ./dev.sh clippy           -> cargo clippy
#   ./dev.sh rebuild-image    -> rebuild the dev image
set -euo pipefail
cd "$(dirname "$0")"
IMAGE=quicknotes-dev

if [[ "${1:-}" == "rebuild-image" ]]; then
    docker build -t "$IMAGE" docker/
    exit 0
fi
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
    docker build -t "$IMAGE" docker/
fi

args=("$@")
[[ ${#args[@]} -eq 0 ]] && args=(build --release)

mkdir -p .docker-cargo
docker run --rm -t \
    -u "$(id -u):$(id -g)" \
    -v "$PWD:/src" \
    -e CARGO_HOME=/src/.docker-cargo \
    -w /src \
    "$IMAGE" cargo "${args[@]}"
