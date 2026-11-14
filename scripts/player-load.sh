#!/usr/bin/env bash
# Runs one player bench and samples the load it puts on the machine.
#
#   scripts/player-load.sh <mpv|fvp> <url>
#
# Build the bench first:
#   flutter build linux --profile -t bench/player_bench.dart
# The environment passes through: JITTER_HWDEC, JITTER_BENCH_FVP_DECODERS,
# JITTER_BENCH_WARMUP and JITTER_BENCH_SECONDS.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
bundle="$root/build/linux/x64/profile/bundle/jitter"

if (( $# != 2 )) || [[ $1 != mpv && $1 != fvp ]]; then
  echo "usage: $0 <mpv|fvp> <url>" >&2
  exit 2
fi
engine="$1"
url="$2"
if [[ ! -x "$bundle" ]]; then
  echo "$0: no bundle at $bundle; build it with:" >&2
  echo "  flutter build linux --profile -t bench/player_bench.dart" >&2
  exit 2
fi

log="$(mktemp)"
pid=""
cleanup() {
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
  fi
  rm -f "$log"
}
trap cleanup EXIT

# Prints the last lines of the bench log on stderr, then exits 1.
die() {
  echo "$0: $1" >&2
  tail -n 20 "$log" >&2
  exit 1
}

# The bench's utime + stime in clock ticks, or ? when unreadable. The comm
# field can contain spaces, so the stat line is split after its last ')'.
cpu_ticks() {
  local stat
  local -a fields
  stat="$(cat "/proc/$pid/stat" 2>/dev/null || true)"
  read -r -a fields <<<"${stat##*)}" || true
  if (( ${#fields[@]} >= 13 )); then
    echo $(( fields[11] + fields[12] ))
  else
    echo "?"
  fi
}

# The bench's resident set in kB, or nothing when unreadable.
rss_kb() {
  awk '/^VmRSS:/ { print $2 }' "/proc/$pid/status" 2>/dev/null || true
}

# The bench's engine time in ns, summed over its unique i915 DRM clients, as
# "render video", or "? ?" when it has no i915 client. Fds that share a
# drm-client-id show the same counters, so each id counts once. Non-DRM and
# unreadable fds are skipped. awk resets its per-fd fields at each file's
# first line, so one awk call covers every fd.
i915_ns() {
  local out
  out="$(awk '
    function add() {
      if (drv == "i915" && id != "" && !(id in seen)) {
        seen[id] = 1
        found = 1
        render += er
        video += ev
      }
    }
    FNR == 1 {
      if (NR > 1) add()
      drv = ""; id = ""; er = 0; ev = 0
    }
    $1 == "drm-driver:" { drv = $2 }
    $1 == "drm-client-id:" { id = $2 }
    $1 == "drm-engine-render:" { er = $2 }
    $1 == "drm-engine-video:" { ev = $2 }
    END {
      add()
      if (found) { print render, video } else { print "? ?" }
    }
  ' /proc/"$pid"/fdinfo/* 2>/dev/null || true)"
  echo "${out:-? ?}"
}

# One sample: VmRSS kB, then nvidia-smi's GPU %, decoder % and power W.
sample() {
  local gpu="?" dec="?" power="?" line rss
  if command -v nvidia-smi >/dev/null 2>&1; then
    line="$(nvidia-smi \
      --query-gpu=utilization.gpu,utilization.decoder,power.draw \
      --format=csv,noheader,nounits 2>/dev/null || true)"
    read -r gpu dec power <<<"${line//,/ }" || true
  fi
  rss="$(rss_kb)"
  echo "${rss:-?} ${gpu:-?} ${dec:-?} ${power:-?}"
}

# The bench's cpu ticks, the wall clock in ms, and its i915 render and video
# engine ns.
snapshot() {
  echo "$(cpu_ticks) $(date +%s%3N) $(i915_ns)"
}

JITTER_BENCH_ENGINE="$engine" JITTER_BENCH_URL="$url" \
  "$bundle" >"$log" 2>&1 &
pid=$!

# Poll every 0.2 s, up to 120 s, for the start of the measured window.
polls=0
until grep -q '^BENCH_START' "$log"; do
  kill -0 "$pid" 2>/dev/null || die "bench exited before BENCH_START"
  (( polls < 600 )) || die "no BENCH_START after 120 s"
  polls=$((polls + 1))
  sleep 0.2
done
read -r ticks0 wall0 render0 video0 <<<"$(snapshot)" || true

# Poll every 0.2 s and sample every fifth poll, which is once a second. The
# bench exits right after BENCH_END, so the end readings are the newest
# snapshot taken while it was still alive, not one taken after the poll
# sees the line.
samples=""
end=""
polls=0
until grep -q '^BENCH_END' "$log"; do
  kill -0 "$pid" 2>/dev/null || die "bench exited before BENCH_END"
  if (( polls % 5 == 0 )); then
    samples+="$(sample)"$'\n'
  fi
  now="$(snapshot)"
  if [[ "${now%% *}" != "?" ]]; then end="$now"; fi
  polls=$((polls + 1))
  sleep 0.2
done
read -r ticks1 wall1 render1 video1 <<<"$end" || true
hz="$(getconf CLK_TCK)"

printf '%s' "$samples" | awk \
  -v engine="$engine" -v hz="$hz" \
  -v ticks0="$ticks0" -v ticks1="$ticks1" \
  -v wall0="$wall0" -v wall1="$wall1" \
  -v render0="$render0" -v render1="$render1" \
  -v video0="$video0" -v video1="$video1" '
function num(s) {
  return s ~ /^[0-9]+(\.[0-9]+)?$/
}
function avg(sum, n) {
  return n > 0 ? sprintf("%.1f", sum / n) : "?"
}
# Percent of the window that a cumulative engine counter in ns advanced
# (dw is in ms, so dw * 1000000 is ns).
function busy(before, after) {
  return num(before) && num(after) && dw > 0 \
    ? sprintf("%.1f", 100 * (after - before) / (dw * 1000000)) : "?"
}
{
  if (num($1)) {
    kb = $1 / 1024
    rss += kb
    if (rn == 0 || kb > rmax) rmax = kb
    rn++
  }
  if (num($2)) { gpu += $2; gn++ }
  if (num($3)) { dec += $3; dn++ }
  if (num($4)) { pw += $4; pn++ }
}
END {
  dw = wall1 - wall0
  cpu = "?"
  if (num(ticks0) && num(ticks1) && dw > 0) {
    cpu = sprintf("%.1f", (ticks1 - ticks0) / hz / (dw / 1000) * 100)
  }
  printf "LOAD engine=%s cpu_pct=%s rss_avg_mb=%s rss_max_mb=%s ", \
    engine, cpu, avg(rss, rn), (rn > 0 ? sprintf("%.1f", rmax) : "?")
  printf "igpu_render_pct=%s igpu_video_pct=%s ", \
    busy(render0, render1), busy(video0, video1)
  printf "nv_gpu_pct=%s nv_dec_pct=%s nv_power_w=%s\n", \
    avg(gpu, gn), avg(dec, dn), avg(pw, pn)
}'

grep '^BENCH_END' "$log"
grep -E 'MDK .*(opening video decoder|failed to setup video decoder)' "$log" \
  || true

status=0
wait "$pid" || status=$?
if (( status != 0 )); then
  echo "$0: bench exited $status after BENCH_END" >&2
  exit 1
fi
