#!/usr/bin/env bash
# auto_brightness.sh — Reads ambient lux and sets brightness via kscreen-doctor

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────
SENSOR_PATH="/sys/bus/iio/devices/iio:device0/in_illuminance_raw"
LUX_MAX=500             # Lux level for maximum brightness
BRIGHTNESS_MIN=10       # Minimum brightness percentage
BRIGHTNESS_MAX=100      # Maximum brightness percentage
BRIGHTNESS_THRESHOLD=5  # Minimum change required to trigger update
INTERVAL=5              # Polling interval in seconds

# ── State Variables ──────────────────────────────────────────────────────────
LAST_SET_BRIGHTNESS=-1

# ── Logic ────────────────────────────────────────────────────────────────────

check_sensor() {
  if [[ ! -r "$SENSOR_PATH" ]]; then
    echo "CRITICAL ERROR: Sensor not found or not readable at $SENSOR_PATH" >&2
    exit 1
  fi
}

update_brightness() {
  # Read raw sensor value
  if ! RAW_LUX=$(cat "$SENSOR_PATH" 2>/dev/null); then
    return 1
  fi

  # Calculate Target Brightness using logarithmic scaling
  TARGET_BRIGHTNESS=$(awk -v lux="$RAW_LUX" -v mx="$LUX_MAX" -v b_min="$BRIGHTNESS_MIN" -v b_max="$BRIGHTNESS_MAX" 'BEGIN {
    v = (lux < 0) ? 0 : (lux > mx) ? mx : lux
    ratio = log(v + 1) / log(mx + 1)
    b = ratio * (b_max - b_min) + b_min
    printf "%d", int(b + 0.5)
  }')

  # Only update if the change exceeds the threshold
  DELTA=$(( TARGET_BRIGHTNESS - LAST_SET_BRIGHTNESS ))
  ABS_DELTA=${DELTA#-}

  if [[ "$ABS_DELTA" -lt "$BRIGHTNESS_THRESHOLD" ]]; then
    return 0
  fi

  # Apply brightness to all outputs via kscreen-doctor
  # We fetch output names dynamically in case of hot-plugging
  for name in $(kscreen-doctor -o | grep "Output: " | awk '{print $3}'); do
    if kscreen-doctor output."$name".brightness."$TARGET_BRIGHTNESS" > /dev/null 2>&1; then
       echo "$(date '+%H:%M:%S') - $name: → ${TARGET_BRIGHTNESS}% (Lux: ${RAW_LUX})"
    fi
  done

  LAST_SET_BRIGHTNESS="$TARGET_BRIGHTNESS"
}

# ── Main ─────────────────────────────────────────────────────────────────────
check_sensor
trap "exit" SIGINT SIGTERM

while true; do
  update_brightness || true
  sleep "$INTERVAL"
done