#!/usr/bin/env bash
# auto_brightness.sh — Reads ambient lux from an IIO light sensor and sets
# monitor brightness via ddcutil using perceptual (logarithmic) scaling.

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────
SENSOR_PATH="/sys/bus/iio/devices/iio:device0/in_illuminance_raw"
LUX_MAX=500             # At this Lux level, the monitor reaches max configured brightness
BRIGHTNESS_MIN=0        # Minimum value for auto brightness.
BRIGHTNESS_MAX=100      # Maximum value for auto brightness
BRIGHTNESS_THRESHOLD=5  # Minimum difference from current brightness to apply a change
INTERVAL=5              # Polling interval
VCP_BRIGHTNESS=10       

# ── State Variables ──────────────────────────────────────────────────────────
LAST_ACTIVE_COUNT=-1
LAST_SET_BRIGHTNESS=-1
MONITOR_IDS=()

# ── Logic ────────────────────────────────────────────────────────────────────

get_active_count() {
  # Sums the '1's (enabled) and '0's (disabled) into a single integer
  cat /sys/class/drm/*/enabled 2>/dev/null | grep -c "enabled" || echo 0
}

refresh_monitor_list() {
  echo "$(date '+%H:%M:%S') - Display state change. Looking for available monitors..."
  # Allow monitors to settle after a state change
  sleep 5
  # Only store numeric IDs to avoid parsing issues
  mapfile -t MONITOR_IDS < <(ddcutil detect --brief 2>/dev/null | awk '/^Display [0-9]+/{print $2}')
}

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
  CURRENT_ACTIVE_COUNT=$(get_active_count)
  FORCE_REFRESH=false

  if [[ "$CURRENT_ACTIVE_COUNT" -ne "$LAST_ACTIVE_COUNT" ]]; then
    refresh_monitor_list
    LAST_ACTIVE_COUNT="$CURRENT_ACTIVE_COUNT"
    FORCE_REFRESH=true 
  fi

  if [[ ${#MONITOR_IDS[@]} -eq 0 ]]; then
    return 0
  fi

  if ! RAW_LUX=$(cat "$SENSOR_PATH" 2>/dev/null); then
    return 1
  fi

  TARGET_BRIGHTNESS=$(awk -v lux="$RAW_LUX" -v mx="$LUX_MAX" -v b_min="$BRIGHTNESS_MIN" -v b_max="$BRIGHTNESS_MAX" 'BEGIN {
    v = (lux < 0) ? 0 : (lux > mx) ? mx : lux
    ratio = log(v + 1) / log(mx + 1)
    b = ratio * (b_max - b_min) + b_min
    printf "%d", int(b + 0.5)
  }')

  DELTA=$(( TARGET_BRIGHTNESS - LAST_SET_BRIGHTNESS ))
  ABS_DELTA=${DELTA#-}

  if [[ "$FORCE_REFRESH" == "false" ]] && [[ "$ABS_DELTA" -lt "$BRIGHTNESS_THRESHOLD" ]]; then
    return 0
  fi

  for DISP in "${MONITOR_IDS[@]}"; do
    if ddcutil --display "$DISP" setvcp "$VCP_BRIGHTNESS" "$TARGET_BRIGHTNESS" 2>/dev/null; then
        echo "$(date '+%H:%M:%S') - Display $DISP: → ${TARGET_BRIGHTNESS}% (Lux: ${RAW_LUX})"
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
