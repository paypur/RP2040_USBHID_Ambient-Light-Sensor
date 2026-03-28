#!/usr/bin/env bash
# auto_brightness.sh — Reads ambient lux from an IIO light sensor and sets
# monitor brightness via ddcutil using perceptual (logarithmic) scaling.

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────
SENSOR_PATH="/sys/bus/iio/devices/iio:device0/in_illuminance_raw"
LUX_MAX=500             # At this Lux level, the monitor reaches max configured brightness
BRIGHTNESS_MIN=5        # Minimum value for auto brightness.
BRIGHTNESS_MAX=100      # Maximum value for auto brightness
BRIGHTNESS_THRESHOLD=5  # Minimum difference from current brightness to apply a change
INTERVAL=15              # Polling interval
# ─────────────────────────────────────────────────────────────────────────────

DRY_RUN=false
VERBOSE=false
VCP_BRIGHTNESS=10       # DDC code for brightness control. Should not change

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=true ;;
    --verbose) VERBOSE=true ;;
    *)         echo "Unknown argument: $arg" >&2; exit 1 ;;
  esac
done


check_sensor() {
  if [[ ! -f "$SENSOR_PATH" ]]; then
    echo "CRITICAL ERROR: Sensor device not found at $SENSOR_PATH" >&2
    echo "Possible fixes:" >&2
    echo "  1. Check if the IIO kernel module (e.g., 'hid_sensor_als') is loaded." >&2
    echo "  2. Verify the path. Your device might be 'iio:device1' instead of 'device0'." >&2
    exit 1
  fi

  if [[ ! -r "$SENSOR_PATH" ]]; then
    echo "CRITICAL ERROR: Sensor found but not readable." >&2
    echo "Try running with 'sudo' or adding your user to the 'iio' or 'video' group." >&2
    exit 1
  fi
}

update_brightness() {
  if ! RAW_LUX=$(cat "$SENSOR_PATH" 2>/dev/null); then
    echo "WARNING: Failed to read sensor during this cycle. Skipping..." >&2
    return 1
  fi

  # Map and Clamp
  LUX=$(awk -v v="$RAW_LUX" -v mx="$LUX_MAX" 'BEGIN {
    v = (v < 0) ? 0 : (v > mx) ? mx : v
    printf "%.4f", v
  }')

  BRIGHTNESS=$(awk -v lux="$LUX" -v lux_max="$LUX_MAX" -v b_min="$BRIGHTNESS_MIN" -v b_max="$BRIGHTNESS_MAX" 'BEGIN {
    ratio = log(lux + 1) / log(lux_max + 1)
    b = ratio * (b_max - b_min) + b_min
    b = (b < b_min) ? b_min : (b > b_max) ? b_max : b
    printf "%d", int(b + 0.5)
  }')

  # Discover and Update Monitors
  mapfile -t DISPLAYS < <(ddcutil detect --brief 2>/dev/null | awk '/^Display [0-9]+/{print $2}')

  for DISP in "${DISPLAYS[@]}"; do
    if "$DRY_RUN"; then
      log "[DRY-RUN] Target: ${BRIGHTNESS}% for Display $DISP"
      continue
    fi

    VCP_OUTPUT=$(ddcutil --display "$DISP" getvcp "$VCP_BRIGHTNESS" 2>/dev/null || true)
    CURRENT_BRIGHTNESS=$(echo "$VCP_OUTPUT" | awk 'match($0, /current value *= *([0-9]+)/, a) {print a[1]}')

    if [[ -n "$CURRENT_BRIGHTNESS" ]]; then
      DELTA=$(awk -v new="$BRIGHTNESS" -v cur="$CURRENT_BRIGHTNESS" 'BEGIN { d = new - cur; printf "%d", (d < 0) ? -d : d }')
      if [[ "$DELTA" -le "$BRIGHTNESS_THRESHOLD" ]]; then
        continue
      fi
    fi

    ddcutil --display "$DISP" setvcp "$VCP_BRIGHTNESS" "$BRIGHTNESS" 2>/dev/null || true
    echo "$(date '+%H:%M:%S') - Display $DISP: ${CURRENT_BRIGHTNESS:-?} → ${BRIGHTNESS}% (Lux: ${RAW_LUX})"
  done
}

# ── Main ─────────────────────────────────────────────────────────────────────
check_sensor

echo "Starting auto-brightness daemon..."
echo "Sensor: $SENSOR_PATH"
echo "Interval: ${INTERVAL}s"
trap "echo 'Shutting down...'; exit" SIGINT SIGTERM

while true; do
  update_brightness || true
  sleep "$INTERVAL"
done
