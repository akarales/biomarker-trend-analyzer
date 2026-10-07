#!/usr/bin/env bash
# Regenerate demo/synthea-subset.json from a pinned Synthea release.
#
#   scripts/synthea/generate_synthea.sh            # from the repo root
#
# Needs Java 17+ and uv. Downloads the release jar once into .cache/
# (gitignored) and verifies its sha256. Same version + seed + reference
# date + keep module ⇒ the same population and the same committed subset.
# Synthea (MITRE) is Apache-2.0; its output is synthetic — no real patients.
set -euo pipefail

SYNTHEA_VERSION=v4.0.0            # released 2026-03-05 (≥ 7 days before pinning)
SYNTHEA_SHA256=ed43c20ad40ba5c3bc724503a5af032715fe3c491620b766148e7c2361e6ecc1
SEED=20261006                     # population + clinician seed
REFERENCE_DATE=20261001           # "today" inside the simulation
POPULATION=40                     # living patients matching the keep module
AGES=40-80
STATE=Massachusetts

root=$(cd "$(dirname "$0")/../.." && pwd)
cache="$root/.cache/synthea"
jar="$cache/synthea-$SYNTHEA_VERSION.jar"
out="$cache/out"
mkdir -p "$cache"

if [[ ! -f "$jar" ]]; then
  curl -sSL -o "$jar.part" \
    "https://github.com/synthetichealth/synthea/releases/download/$SYNTHEA_VERSION/synthea-with-dependencies.jar"
  mv "$jar.part" "$jar"
fi
echo "$SYNTHEA_SHA256  $jar" | sha256sum --check --quiet

rm -rf "$out"
java -jar "$jar" \
  -s "$SEED" -cs "$SEED" -p "$POPULATION" -a "$AGES" -r "$REFERENCE_DATE" \
  -k "$root/scripts/synthea/keep_conditions.json" \
  --exporter.baseDirectory="$out" \
  --exporter.fhir.export=true \
  --exporter.hospital.fhir.export=false \
  --exporter.practitioner.fhir.export=false \
  --exporter.years_of_history=10 \
  "$STATE" > "$cache/run.log"
tail -n 4 "$cache/run.log" | head -n 1

uv run --no-project "$root/scripts/synthea/select_subset.py" "$out/fhir" "$root/demo/synthea-subset.json"
