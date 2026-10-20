#!/bin/sh
# Profiles native composition of one corpus case with macOS `sample` and prints where the
# time goes: by category (allocation, hashing...) and by function, inclusive of callees.
#
# Usage: bench/profile.sh [case-id] [seconds]    (default: fixture:huge-schema 15)
# Set FEATURES=mimalloc to profile with mimalloc as the global allocator.
set -e
CASE=${1:-fixture:huge-schema}
SECONDS_TO_SAMPLE=${2:-15}
OUT=target/profile-$(echo "$CASE" | tr ':/' '__').txt
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build -q -p federation-composition-rs --release ${FEATURES:+--features "$FEATURES"} --example profile_case
./target/release/examples/profile_case "$CASE" $((SECONDS_TO_SAMPLE + 5)) &
PID=$!
sleep 2
sample "$PID" "$SECONDS_TO_SAMPLE" -file "$OUT" >/dev/null 2>&1
kill "$PID" 2>/dev/null || true
python3 bench/profile_report.py "$OUT"
