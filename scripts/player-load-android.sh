#!/usr/bin/env bash
# Runs one player bench on the Android phone and samples the load it puts on
# the phone: the bench app's CPU and PSS, the whole system's CPU, and what
# SurfaceFlinger drew for the app's window.
#
#   scripts/player-load-android.sh <mpv|fvp|exo> <url>
#
# Install the bench first; this script does not install it:
#   flutter build apk --profile --target-platform android-arm64 \
#     -t bench/player_bench.dart
#   adb install -r build/app/outputs/flutter-apk/app-profile.apk
# The environment passes through: JITTER_BENCH_MPV_HWDEC,
# JITTER_BENCH_FVP_DECODERS, JITTER_BENCH_WARMUP and JITTER_BENCH_SECONDS.
# The bench reads its settings from this script over `adb reverse`, and the
# screen stays on for the run.
set -euo pipefail

pkg="io.github.lumpsoid.jitter.bench"
port=8765

if (( $# != 2 )) || [[ $1 != mpv && $1 != fvp && $1 != exo ]]; then
  echo "usage: $0 <mpv|fvp|exo> <url>" >&2
  exit 2
fi
engine="$1"
url="$2"

if ! command -v adb >/dev/null 2>&1; then
  echo "$0: adb is not on PATH" >&2
  exit 2
fi
devices="$(adb devices |
  awk 'NR > 1 && $2 == "device" { n++ } END { print n + 0 }')"
if (( devices != 1 )); then
  echo "$0: need exactly one adb device, found $devices" >&2
  exit 2
fi

config="$(mktemp)"
log="$(mktemp)"
serve_pid=""
logcat_pid=""
stay_on=""
stay_on_set=0
timestats=0

# Restores what the run changed on the phone and stops what it started.
cleanup() {
  if (( stay_on_set )); then
    if [[ "$stay_on" == null ]]; then
      adb shell settings delete global stay_on_while_plugged_in || true
    else
      adb shell settings put global stay_on_while_plugged_in \
        "$stay_on" || true
    fi
  fi
  if (( timestats )); then
    adb shell dumpsys SurfaceFlinger --timestats -disable >/dev/null || true
  fi
  if [[ -n "$logcat_pid" ]]; then kill "$logcat_pid" 2>/dev/null || true; fi
  if [[ -n "$serve_pid" ]]; then kill "$serve_pid" 2>/dev/null || true; fi
  adb reverse --remove "tcp:$port" >/dev/null 2>&1 || true
  adb shell am force-stop "$pkg" >/dev/null 2>&1 || true
  rm -f "$config" "$log"
}
trap cleanup EXIT

# Prints the last lines of the logcat on stderr, then exits 1.
die() {
  echo "$0: $1" >&2
  tail -n 30 "$log" >&2
  exit 1
}

# Whether the bench printed the marker $1. Its lines come from the flutter
# tag in logcat's brief format.
seen() {
  grep -q "/flutter .*): $1" "$log"
}

# The bench's pid, or nothing when it is not running.
app_pid() {
  adb shell pidof "$pkg" 2>/dev/null | tr -d '\r' || true
}

# One reading: the app's utime + stime in ticks, the system's busy and total
# jiffies, and the host's wall clock in ms. The app's ticks are ? once its
# stat is unreadable, which means the bench has gone. busy is user, nice,
# system, irq, softirq and steal; total adds idle and iowait.
snapshot() {
  local wall out
  wall="$(date +%s%3N)"
  out="$(adb shell "cat /proc/$pid/stat; head -n 1 /proc/stat" \
    2>/dev/null | tr -d '\r' || true)"
  awk -v wall="$wall" '
    BEGIN { ticks = "?"; busy = "?"; total = "?" }
    $1 ~ /^[0-9]+$/ && /\) / { sub(/.*\) /, ""); ticks = $12 + $13 }
    $1 == "cpu" {
      busy = $2 + $3 + $4 + $7 + $8 + $9
      total = busy + $5 + $6
    }
    END { print ticks, busy, total, wall }
  ' <<<"$out"
}

# The app's total PSS in kB, or nothing when dumpsys shows none.
pss_kb() {
  adb shell dumpsys meminfo "$pkg" 2>/dev/null | tr -d '\r' |
    awk '/TOTAL PSS:/ { print $3; exit }' || true
}

# Writes the bench's settings for the run. The bench reads them over the
# reverse socket; empty optional settings are left out, as the bench reads
# them as unset.
{
  printf 'JITTER_BENCH_URL=%s\n' "$url"
  printf 'JITTER_BENCH_ENGINE=%s\n' "$engine"
  for var in JITTER_BENCH_WARMUP JITTER_BENCH_SECONDS \
    JITTER_BENCH_FVP_DECODERS JITTER_BENCH_MPV_HWDEC; do
    if [[ -n "${!var:-}" ]]; then printf '%s=%s\n' "$var" "${!var}"; fi
  done
} >"$config"

# A one-shot server on 127.0.0.1: it sends the config to the first
# connection, then exits. It gives up after 120 s with no connection.
serve_py='
import socket, sys
data = open(sys.argv[1], "rb").read()
server = socket.socket()
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(("127.0.0.1", int(sys.argv[2])))
server.listen(1)
server.settimeout(120)
try:
    conn, _ = server.accept()
except OSError:
    sys.exit(1)
conn.sendall(data)
conn.close()
'
adb reverse "tcp:$port" "tcp:$port" >/dev/null
python3 -I -c "$serve_py" "$config" "$port" &
serve_pid=$!

stay_on="$(adb shell settings get global stay_on_while_plugged_in |
  tr -d '\r')"
stay_on_set=1
adb shell svc power stayon usb
adb shell input keyevent KEYCODE_WAKEUP

adb logcat -c
adb shell am force-stop "$pkg"
start="$(adb shell am start \
  -n "$pkg/io.github.lumpsoid.jitter.MainActivity" 2>&1 |
  tr -d '\r' || true)"
case "$start" in
  *Error*) die "am start failed: $start" ;;
esac
adb logcat -v brief >"$log" 2>/dev/null &
logcat_pid=$!

# Waits up to 120 s for BENCH_START, and fails if the bench errors or exits.
began=$SECONDS
pid=""
until seen BENCH_START; do
  if seen BENCH_ERROR; then die "bench failed before BENCH_START"; fi
  now_pid="$(app_pid)"
  if [[ -n "$now_pid" ]]; then
    pid="$now_pid"
  elif [[ -n "$pid" ]]; then
    die "bench exited before BENCH_START"
  fi
  if (( SECONDS - began >= 120 )); then die "no BENCH_START after 120 s"; fi
  sleep 0.5
done

read -r ticks0 busy0 total0 wall0 <<<"$(snapshot)" || true
adb shell dumpsys SurfaceFlinger --timestats -clear -enable >/dev/null
timestats=1

# Samples until BENCH_END. Each snapshot is taken just before the log is
# checked, so the end reading is the last one taken while BENCH_END was
# unseen, and the teardown after the line does not count (as in
# player-load.sh). Memory is read every 5 s.
pss_sum=0
pss_n=0
pss_max=0
mem_at=-5
ticks1="?"
busy1="?"
total1="?"
wall1="?"
while :; do
  read -r now_ticks now_busy now_total now_wall <<<"$(snapshot)" || true
  if seen BENCH_END; then break; fi
  if seen BENCH_ERROR; then die "bench failed after BENCH_START"; fi
  if [[ "$now_ticks" == "?" ]]; then
    sleep 1
    seen BENCH_END || die "bench exited before BENCH_END"
    break
  fi
  if (( SECONDS - began > 600 )); then die "no BENCH_END after 600 s"; fi
  ticks1=$now_ticks
  busy1=$now_busy
  total1=$now_total
  wall1=$now_wall
  if (( SECONDS - mem_at >= 5 )); then
    mem_at=$SECONDS
    kb="$(pss_kb)"
    if [[ "$kb" =~ ^[0-9]+$ ]]; then
      pss_sum=$(( pss_sum + kb ))
      pss_n=$(( pss_n + 1 ))
      if (( kb > pss_max )); then pss_max=$kb; fi
    fi
  fi
  sleep 0.5
done

sf_dump="$(adb shell dumpsys SurfaceFlinger --timestats -dump |
  tr -d '\r' || true)"
adb shell dumpsys SurfaceFlinger --timestats -disable >/dev/null
timestats=0

hz="$(adb shell getconf CLK_TCK 2>/dev/null | tr -d '\r' || true)"
[[ "$hz" =~ ^[0-9]+$ ]] || hz=100

load="$(awk \
  -v engine="$engine" -v hz="$hz" \
  -v ticks0="$ticks0" -v ticks1="$ticks1" \
  -v busy0="$busy0" -v busy1="$busy1" \
  -v total0="$total0" -v total1="$total1" \
  -v wall0="$wall0" -v wall1="$wall1" \
  -v pss_sum="$pss_sum" -v pss_n="$pss_n" -v pss_max="$pss_max" '
function num(s) {
  return s ~ /^[0-9]+$/
}
BEGIN {
  app = "?"
  sys = "?"
  dwall = (wall1 - wall0) / 1000
  if (num(ticks0) && num(ticks1) && dwall > 0) {
    app = sprintf("%.1f", (ticks1 - ticks0) / hz / dwall * 100)
  }
  if (num(busy0) && num(busy1) && num(total0) && num(total1)) {
    dtotal = total1 - total0
    if (dtotal > 0) sys = sprintf("%.1f", (busy1 - busy0) / dtotal * 100)
  }
  avg = pss_n > 0 ? sprintf("%.1f", pss_sum / pss_n / 1024) : "?"
  max = pss_n > 0 ? sprintf("%.1f", pss_max / 1024) : "?"
  printf "LOAD engine=%s cpu_app_pct=%s cpu_sys_pct=%s ", engine, app, sys
  printf "pss_avg_mb=%s pss_max_mb=%s", avg, max
}')"

# The bench's layer in SurfaceFlinger's timestats: the one with the most
# frames among the layers named for the app, or ? when there is none.
sf="$(awk '
  function flush() {
    if (cur && (!best || frames + 0 > bframes + 0)) {
      best = 1
      bframes = frames
      bdropped = dropped
      bjanky = janky
      bfps = fps
    }
    cur = 0
  }
  /^layerName = / {
    flush()
    cur = index($0, "io.github.lumpsoid.jitter") > 0
    frames = 0
    dropped = "?"
    janky = "?"
    fps = "?"
    next
  }
  /^totalFrames = / { frames = $3 }
  /^droppedFrames = / { dropped = $3 }
  /^jankyFrames = / { janky = $3 }
  /^averageFPS = / { fps = $3 }
  END {
    flush()
    if (best) {
      printf "sf_fps=%s sf_frames=%s sf_dropped=%s sf_janky=%s\n", \
        bfps, bframes, bdropped, bjanky
    } else {
      print "sf_fps=? sf_frames=? sf_dropped=? sf_janky=?"
    }
  }
' <<<"$sf_dump")"

echo "$load $sf"
grep -m 1 -o 'BENCH_END .*' "$log" || true
# The decoder names differ by engine: mpv logs ACodec's OMX component and
# VideoDec, and exo logs its MediaCodec and CCodec components.
decoder='Created component \[|MediaCodec.*(Created|configure)|AMediaCodec'
decoder+='|ACodec.*decoder|OMX\.[a-z.]*decoder|c2\.[a-z0-9.]*decoder'
decoder+='|VideoDec\[|opening video decoder'
grep -E "$decoder" "$log" | sed -E 's/^[^:]*: //' |
  awk '!dup[$0]++' | head -n 5 || true
