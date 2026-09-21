#!/usr/bin/env bash
#
# Run the differential fuzz suites across all cores.
#
# `cargo test` runs each test function as a *thread* in one process, so a single
# long-running test (a case with a large `|y·ln x|`, say) pins one core while the rest
# sit idle. This runner gives every test — and, with `-s`, every shard of a test — its
# own process, and keeps a pool of them busy.
#
#   fuzz/run.sh                      # whole suite, one process per test
#   fuzz/run.sh -c 4096              # 4096 cases per test (default 1024)
#   fuzz/run.sh -s 8 powf            # split the powf test 8 ways
#   fuzz/run.sh -j 8 -s 4            # 8 concurrent processes, 4 shards per test
#   FUZZ_PRECISIONS=503 fuzz/run.sh  # a single precision width
#
# Options:
#   -j JOBS        concurrent processes           (default: nproc)
#   -s SHARDS      shards per test                (default: 1)
#   -c CASES       case budget per test, FUZZ_CASES   (default: 1024)
#   -p PRECISIONS  width sweep in bits, FUZZ_PRECISIONS
#   -S SEED        pin the RNG for reproducibility (FUZZ_SEED)
#   -k             keep each job's log even on success (default: failures only)
#   -v             echo every test as it starts
#
# Positional arguments are substring filters matched against `<binary>::<test>`;
# with none, everything that is `#[ignore]`d runs.
#
# Shards split a test's case budget and take distinct RNG seeds, so `-s N` keeps the
# coverage of one run while using N cores — see `fuzz::fuzz_config`.

set -uo pipefail

self="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
here="$(dirname "$self")"
cd "$here"

JOBS="$( (nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4) )"
SHARDS=1
KEEP_LOGS=0
VERBOSE=0
FILTERS=()
# The documented case-count knob is `FUZZ_CASES`; `PROPTEST_CASES` is the fallback, exactly as
# `fuzz::fuzz_config` reads them (defaulting it here instead would silently shadow it).
export FUZZ_CASES="${FUZZ_CASES:-${PROPTEST_CASES:-1024}}"

usage() { sed -n '3,26p' "$self" | sed 's/^# \{0,1\}//'; }

# An option that takes a value needs one more argument (`set -u` would die on `$2` with an
# opaque "unbound variable" otherwise).
need_val() { (($# >= 2)) || { echo "option $1 needs a value" >&2; exit 2; }; }
while (($#)); do
    case "$1" in
        -j) need_val "$@"; JOBS="$2"; shift 2 ;;
        -s) need_val "$@"; SHARDS="$2"; shift 2 ;;
        -c) need_val "$@"; FUZZ_CASES="$2"; shift 2 ;;
        -p) need_val "$@"; FUZZ_PRECISIONS="$2"; export FUZZ_PRECISIONS; shift 2 ;;
        -S) need_val "$@"; FUZZ_SEED="$2"; export FUZZ_SEED; shift 2 ;;
        -k) KEEP_LOGS=1; shift ;;
        -v) VERBOSE=1; shift ;;
        -h|--help) usage; exit 0 ;;
        --) shift; FILTERS+=("$@"); break ;;
        -*) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
        *) FILTERS+=("$1"); shift ;;
    esac
done
export FUZZ_CASES

# validate the numeric knobs (flag name, variable, value)
for spec in "j:JOBS" "s:SHARDS" "c:FUZZ_CASES"; do
    flag="${spec%%:*}"; var="${spec##*:}"
    if ! [[ "${!var}" =~ ^[1-9][0-9]*$ ]]; then
        echo "-$flag (env ${var}) must be a positive integer (got '${!var}')" >&2
        exit 2
    fi
done

# The precision sweep is a comma-separated list of widths; a zero (or unparseable) entry
# degrades the differentials to no check at all, so validate each one.
if [[ -n "${FUZZ_PRECISIONS:-}" ]]; then
    IFS=',' read -r -a widths <<<"$FUZZ_PRECISIONS"
    for w in "${widths[@]}"; do
        if ! [[ "${w// /}" =~ ^[1-9][0-9]*$ ]]; then
            echo "-p (env FUZZ_PRECISIONS) must be comma-separated positive integers (got '$FUZZ_PRECISIONS')" >&2
            exit 2
        fi
    done
fi

# --- build once, so the pool never contends on cargo's build lock -------------
echo "building fuzz test binaries..."
build_out="$(cargo test --release --no-run 2>&1)"
build_rc=$?
if ((build_rc != 0)); then
    printf '%s\n' "$build_out" >&2
    echo "build failed (exit $build_rc)" >&2
    exit 1
fi
# `Executable tests/<src>.rs (<path>)` — the test binaries, not the lib unittests.
BINS=()
while IFS= read -r path; do
    [[ -n "$path" ]] && BINS+=("$path")
done < <(printf '%s\n' "$build_out" | sed -n 's/^ *Executable tests\/.* (\(.*\))$/\1/p')
if ((${#BINS[@]} == 0)); then
    echo "no test binaries found in the build output" >&2
    exit 1
fi

# --- enumerate the ignored tests, expanded into (test × shard) jobs -----------
JOBS_LIST=()
for bin in "${BINS[@]}"; do
    # A listing that fails (a binary that cannot start, a missing shared library) must be loud:
    # swallowing it would drop every test in that binary and still report success.
    list_out="$("$bin" --ignored --list 2>&1)"
    list_rc=$?
    if ((list_rc != 0)); then
        printf '%s\n' "$list_out" >&2
        echo "cannot list the tests of $bin (exit $list_rc)" >&2
        exit 1
    fi
    while IFS= read -r line; do
        name="${line%: test}"
        [[ "$name" == "$line" || -z "$name" ]] && continue
        id="$(basename "$bin")::$name"
        if ((${#FILTERS[@]})); then
            matched=0
            for f in "${FILTERS[@]}"; do
                [[ "$id" == *"$f"* ]] && { matched=1; break; }
            done
            ((matched)) || continue
        fi
        for ((s = 0; s < SHARDS; s++)); do
            JOBS_LIST+=("$bin"$'\t'"$name"$'\t'"$s")
        done
    done <<<"$list_out"
done

if ((${#JOBS_LIST[@]} == 0)); then
    echo "no ignored tests matched ${FILTERS[*]:-<all>}" >&2
    exit 1
fi

echo "running ${#JOBS_LIST[@]} job(s): $JOBS concurrent, $SHARDS shard(s) per test, ${FUZZ_CASES} cases"

# --- run them through a pool of $JOBS ----------------------------------------
# `wait -n` is the pool's throttle, but it needs bash ≥ 4.3 — macOS still ships 3.2, where it
# fails with "invalid option" and, since the status is ignored, the pool would launch every job
# at once. Probe once; without it, fall back to draining the pool fully on each refill.
HAVE_WAIT_N=0
if ( : & wait -n ) 2>/dev/null; then
    HAVE_WAIT_N=1
else
    echo "note: this bash has no 'wait -n' (needs 4.3+) — the pool drains fully before refilling" >&2
fi
# Checked: every job's log *and* the failure sentinel live here, so a failure to create it would
# otherwise make every job fail its redirect and still report "all passed" (there is no `set -e`).
LOGDIR="$(mktemp -d "${TMPDIR:-/tmp}/fuzz-run.XXXXXX")" || {
    echo "cannot create a log directory under ${TMPDIR:-/tmp}" >&2
    exit 1
}
trap '((KEEP_LOGS)) || rm -rf "$LOGDIR"' EXIT
run_job() {
    local bin="$1" name="$2" shard="$3"
    local log="$LOGDIR/$(basename "$bin").$name.$shard.log"
    ((VERBOSE)) && echo "  -> $name (shard $shard)"
    if ! FUZZ_SHARDS="$SHARDS" FUZZ_SHARD="$shard" \
        "$bin" --ignored --exact "$name" >"$log" 2>&1; then
        # The job already wrote its own failure detail to $log; surface it once the pool drains.
        echo "$log" >>"$LOGDIR/failed"
        echo "FAIL  $name (shard $shard)"
    elif ((!KEEP_LOGS)); then
        rm -f "$log"
    fi
}

start=$SECONDS
busy=0
for desc in "${JOBS_LIST[@]}"; do
    IFS=$'\t' read -r bin name shard <<<"$desc"
    run_job "$bin" "$name" "$shard" &
    ((busy++))
    if ((busy >= JOBS)); then
        if ((HAVE_WAIT_N)); then
            wait -n
            ((busy--))
        else
            wait
            busy=0
        fi
    fi
done
wait
elapsed=$((SECONDS - start))

# --- report -------------------------------------------------------------------
if [[ -f "$LOGDIR/failed" ]]; then
    echo
    echo "===== failure detail ====="
    while IFS= read -r log; do
        echo "--- $(basename "$log" .log) ---"
        grep -v '^$' "$log" | tail -40
    done <"$LOGDIR/failed"
    echo
    if ((KEEP_LOGS)); then
        echo "logs kept in $LOGDIR"
    fi
    echo "FAILED in ${elapsed}s — $(wc -l <"$LOGDIR/failed") failing job(s)"
    exit 1
fi

if ((KEEP_LOGS)); then
    echo "logs kept in $LOGDIR"
fi
echo "all ${#JOBS_LIST[@]} job(s) passed in ${elapsed}s"
