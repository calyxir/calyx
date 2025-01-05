#!/bin/bash

HARDFLOAT_DIR="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

HARDFLOAT_URL="http://www.jhauser.us/arithmetic/HardFloat-1.zip"

ZIP_FILE="HardFloat-1.zip"

cd "${HARDFLOAT_DIR}"

curl -LO "${HARDFLOAT_URL}"

if [ -f "$ZIP_FILE" ]; then
    unzip -o "$ZIP_FILE"
    echo "HardFloat library fetched and extracted to ${HARDFLOAT_DIR}"
else
    echo "Failed to download HardFloat library from ${HARDFLOAT_URL}"
    exit 1
fi
