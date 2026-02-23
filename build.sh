#!/bin/bash

# Build script for RP2040 ALS HID Sensor

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [ -z "$PICO_SDK_PATH" ]; then
    DEFAULT_SDK="$SCRIPT_DIR/pico-sdk"
    if [ -d "$DEFAULT_SDK" ]; then
        export PICO_SDK_PATH="$DEFAULT_SDK"
    else
        echo "Downloading pico sdk..."
        git clone https://github.com/raspberrypi/pico-sdk.git
        cd pico-sdk
        git submodule update --init
        cd ..
        export PICO_SDK_PATH="$DEFAULT_SDK"
    fi
fi

echo "Building RP2040 ALS HID Sensor..."
echo "Using Pico SDK at: $PICO_SDK_PATH"

# Create build directory
if [ ! -d "build" ]; then
    mkdir build
fi

cd build

# Configure with CMake
echo "Configuring with CMake..."
cmake ..

# Build the project
echo "Building..."
make -j$(nproc)

if [ $? -eq 0 ]; then
    echo ""
    echo "Build completed successfully!"
    echo "Output file: build/als_hid_sensor.uf2"
    echo ""
    echo "To flash:"
    echo "1. Hold BOOTSEL button while connecting RP2040 to USB"
    echo "2. Copy als_hid_sensor.uf2 to the mounted RPI-RP2 drive"
    echo "3. Device will reboot automatically"
else
    echo "Build failed!"
    exit 1
fi