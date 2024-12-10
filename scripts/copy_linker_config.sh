#!/bin/bash

UNAME_MACHINE="$(uname -m)"
LINKER_CONFIG="scripts/linker_config/${UNAME_MACHINE,,}.toml"

if [ -f "${LINKER_CONFIG}" ]; then
    mkdir -p "$HOME/.cargo"
    cat "${LINKER_CONFIG}" >> "$HOME/.cargo/config.toml"
else
    echo "Linker config for ${UNAME_MACHINE,,} not found"
    exit 1
fi