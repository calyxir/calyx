#!/bin/bash

HARDFLOAT_DIR="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

HARDFLOAT_URL="http://www.jhauser.us/arithmetic/HardFloat-1.zip"

ZIP_FILE="HardFloat-1.zip"

cd "${HARDFLOAT_DIR}"

curl -LO "${HARDFLOAT_URL}"

if [ -f "$ZIP_FILE" ]; then
    unzip -o "$ZIP_FILE"
    echo "HardFloat library fetched and extracted to ${HARDFLOAT_DIR}"
    DEST_DIR="${CALYX_PRIMITIVES_DIR:-$HOME/.calyx}"

    echo "Copying HardFloat to destination directory: $DEST_DIR"
    mkdir -p "$DEST_DIR/primitives/float/HardFloat-1"
    cp -r HardFloat-1/* "$DEST_DIR/primitives/float/HardFloat-1/"
    echo "HardFloat copied successfully to $DEST_DIR/primitives/float/HardFloat-1"
else
    echo "Failed to download HardFloat library from ${HARDFLOAT_URL}"
    exit 1
fi
