#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$project_root"

if [[ -f "${NVM_DIR:-$HOME/.nvm}/nvm.sh" ]]; then
  # Load NVM in non-interactive shells and select the repository's pinned Node.
  source "${NVM_DIR:-$HOME/.nvm}/nvm.sh"
  nvm install
  nvm use
else
  printf '%s\n' 'NVM was not found. Install NVM or activate Node 22.23.1 before running this script.' >&2
  exit 1
fi

corepack enable
corepack prepare pnpm@10.15.0 --activate
pnpm install --frozen-lockfile
pnpm package

app_version=$(node -p 'JSON.parse(require("fs").readFileSync("src-tauri/tauri.conf.json", "utf8")).version')
deb="$project_root/src-tauri/target/release/bundle/deb/Narguis Notes App_${app_version}_amd64.deb"
if [[ ! -f "$deb" ]]; then
  printf 'Package was not produced: %s\n' "$deb" >&2
  exit 1
fi

# apt installs the package in place and upgrades an existing version without an uninstall step.
sudo apt-get install -y --reinstall "$deb"

if [[ "${1:-}" != "--no-launch" ]]; then
  nohup /usr/bin/notes-planner-desktop >/dev/null 2>&1 &
fi

printf '%s\n' 'Narguis Notes App installed/updated successfully.'
if [[ "${1:-}" == "--no-launch" ]]; then
  printf '%s\n' 'Launch skipped (--no-launch).'
else
  printf '%s\n' 'Narguis Notes App is launching.'
fi
