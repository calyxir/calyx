#!/bin/bash

HARDFLOAT_DIR="."

HARDFLOAT_URL="http://www.jhauser.us/arithmetic/HardFloat-1.zip"

ZIP_FILE="HardFloat-1.zip"

rm -rf "${HARDFLOAT_DIR}/HardFloat-1"

mkdir -p "${HARDFLOAT_DIR}"

cd "${HARDFLOAT_DIR}"

curl -LO "${HARDFLOAT_URL}"

if [ -f "$ZIP_FILE" ]; then
    unzip "$ZIP_FILE" && rm "$ZIP_FILE"
    echo "HardFloat library fetched and extracted to ${HARDFLOAT_DIR}"
else
    echo "Failed to download HardFloat library from ${HARDFLOAT_URL}"
fi
