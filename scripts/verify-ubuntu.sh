#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != "" ]]; then
  printf 'Usage: pnpm verify:ubuntu\n' >&2
  exit 2
fi

required_commands=(cargo node pnpm rustc xdotool xvfb-run)
for command in "${required_commands[@]}"; do
  command -v "$command" >/dev/null
done

if [[ -r /etc/os-release ]]; then
  . /etc/os-release
  if [[ "${ID:-}" != "ubuntu" ]]; then
    printf 'Expected Ubuntu 22.04+; found %s\n' "${ID:-unknown}" >&2
    exit 1
  fi
fi

printf 'UBUNTU_PREREQUISITES_OK\n'
