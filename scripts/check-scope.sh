#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'Usage: bash scripts/check-scope.sh <initial-path-list>\n' >&2
  exit 2
fi

initial_paths=$1
if [[ ! -f $initial_paths ]]; then
  printf 'SCOPE_ERROR initial path list not found: %s\n' "$initial_paths" >&2
  exit 2
fi

scope_root=${SCOPE_ROOT:-.}
continuation_hash_manifest=${CONTINUATION_HASH_MANIFEST:-$scope_root/.omo/evidence/initial-continuation-hash.txt}

failures=0
report_failure() {
  printf 'SCOPE_ERROR %s\n' "$1" >&2
  failures=1
}

declare -A baseline_paths=()
continuation_baseline_paths=()
while IFS= read -r status_line; do
  path=${status_line:3}
  baseline_paths["$path"]=1
  [[ $path == .omo/run-continuation/*.json ]] && continuation_baseline_paths+=("$path")
done < "$initial_paths"

forbidden_behavior='\b(sync|oauth|telemetry|reminder|recurr[a-z]*|attachment|search|trash|appimage)\b'
scan_paths=(src src-tauri/src src-tauri/capabilities src-tauri/permissions public)
scan_files=(biome.json index.html package.json playwright.config.ts tsconfig.json vite.config.ts vitest.config.ts vitest.integration.config.ts src-tauri/tauri.conf.json)
for path in "${scan_paths[@]}"; do
  if [[ -d $path ]]; then
    while IFS= read -r match; do
      report_failure "forbidden product behavior: $match"
    done < <(
      grep -RIniE --exclude='*.lock' --exclude-dir=target --exclude-dir=gen "$forbidden_behavior" "$path" |
        grep -v 'sync::' || true
    )
  fi
done
for path in "${scan_files[@]}"; do
  if [[ -f $path ]]; then
    while IFS= read -r match; do
      report_failure "forbidden product behavior: $match"
    done < <(grep -IniE "$forbidden_behavior" "$path" | grep -v 'sync::' || true)
  fi
done

if [[ ! -f $continuation_hash_manifest ]]; then
  report_failure "continuation hash manifest not found: $continuation_hash_manifest"
else
  expected_continuation_hash=$(< "$continuation_hash_manifest")
  actual_continuation_hash=$(
    for continuation_path in "${continuation_baseline_paths[@]}"; do
      absolute_path=$scope_root/$continuation_path
      if [[ ! -f $absolute_path ]]; then
        report_failure "baseline continuation artifact missing: $continuation_path"
        continue
      fi
      sha256sum -- "$absolute_path" | sed "s#  $absolute_path#  $continuation_path#"
    done | LC_ALL=C sort | sha256sum | cut -d ' ' -f1
  )
  if [[ $actual_continuation_hash != "$expected_continuation_hash" ]]; then
    report_failure "baseline continuation artifact content changed"
  fi
fi


while IFS= read -r status_line; do
  path=${status_line:3}
  case $path in
    *.db|*.db-shm|*.db-wal|.env*|*.key|*.pem|*.p12|*.pfx|*signing*|target/*|*/target/*)
      report_failure "forbidden hygiene artifact: $path"
      ;;
  esac
done < <(git status --porcelain=v1 --untracked-files=all)

shopt -s nullglob
for path in .env* *.db *.db-shm *.db-wal; do
  report_failure "forbidden hygiene artifact: $path"
done
if [[ -d target ]]; then
  report_failure "forbidden hygiene artifact: target/"
fi

if (( failures != 0 )); then
  exit 1
fi

printf 'SCOPE_OK initial_paths=%s\n' "$initial_paths"
