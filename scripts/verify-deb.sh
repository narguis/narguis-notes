#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  printf 'Usage: pnpm verify:deb\n' >&2
  exit 2
fi

readonly repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
readonly artifact_directory="$repository_root/artifacts/verification/$timestamp"
readonly package_path="$repository_root/dist/planner.deb"

mkdir -p "$artifact_directory" "$repository_root/dist"
exec > >(tee "$artifact_directory/verify-deb.log") 2>&1

command -v docker >/dev/null

docker run --rm -i \
  --env "HOST_UID=$(id -u)" \
  --env "HOST_GID=$(id -g)" \
  --mount "type=bind,src=$repository_root,dst=/workspace" \
  ubuntu:22.04 \
  bash -s <<'BUILDER'
set -euo pipefail

apt-get update
apt-get install -y --no-install-recommends \
  build-essential ca-certificates curl file libayatana-appindicator3-dev libgtk-3-dev \
  librsvg2-dev libwebkit2gtk-4.1-dev patchelf pkg-config
curl --fail --location https://nodejs.org/dist/v22.23.1/node-v22.23.1-linux-x64.tar.xz \
  | tar --extract --xz --directory /opt
ln -s /opt/node-v22.23.1-linux-x64 /opt/node
curl --fail --location --proto '=https' --tlsv1.2 https://sh.rustup.rs \
  | sh -s -- -y --default-toolchain 1.88.0 --profile minimal
export PATH="/opt/node/bin:/root/.cargo/bin:$PATH"
export COREPACK_ENABLE_DOWNLOAD_PROMPT=0
export CI=true
corepack enable
cd /workspace
pnpm install --frozen-lockfile --force --store-dir /tmp/pnpm-store
pnpm package
chown -R "$HOST_UID:$HOST_GID" /workspace/node_modules /workspace/src-tauri/target /workspace/dist
BUILDER

mapfile -t built_packages < <(
  compgen -G "$repository_root/src-tauri/target/release/bundle/deb/*.deb" || true
)
if [[ ${#built_packages[@]} -ne 1 ]]; then
  printf 'Expected exactly one Debian bundle, found %s\n' "${#built_packages[@]}" >&2
  exit 1
fi
cp "${built_packages[0]}" "$package_path"
cp "$package_path" "$artifact_directory/planner.deb"

docker run --rm -i \
  --mount "type=bind,src=$repository_root,dst=/workspace,readonly" \
  --mount "type=bind,src=$artifact_directory,dst=/artifacts" \
  ubuntu:22.04 \
  bash -s -- /workspace/dist/planner.deb /artifacts/planner-wrong-arch.deb <<'CONTAINER'
set -euo pipefail

readonly package_path="$1"
readonly wrong_architecture_package="$2"
readonly package_name="notes-planner"
readonly expected_architecture="amd64"
readonly xdg_root="/tmp/notes-planner-xdg"

apt-get update
apt-get install -y --no-install-recommends dbus-x11 libgtk-3-bin xauth xdotool xvfb

test -f "$package_path"
test "$(dpkg-deb -f "$package_path" Package)" = "$package_name"
test "$(dpkg-deb -f "$package_path" Architecture)" = "$expected_architecture"

mkdir -p /tmp/wrong-architecture/DEBIAN
dpkg-deb --extract "$package_path" /tmp/wrong-architecture
dpkg-deb --control "$package_path" /tmp/wrong-architecture/DEBIAN
sed -i 's/^Architecture: .*/Architecture: arm64/' /tmp/wrong-architecture/DEBIAN/control
dpkg-deb --build /tmp/wrong-architecture "$wrong_architecture_package"
test "$(dpkg-deb -f "$wrong_architecture_package" Architecture)" = "arm64"

if dpkg -i "$wrong_architecture_package"; then
  printf 'Wrong-architecture package unexpectedly installed\n' >&2
  exit 1
fi
if dpkg-query -W --showformat='${db:Status-Abbrev}' "$package_name" 2>/dev/null; then
  printf 'Wrong-architecture package created an installed package state\n' >&2
  exit 1
fi

apt-get install -y "$package_path"
test "$(dpkg-query -W --showformat='${db:Status-Abbrev}' "$package_name")" = "ii "

binary_path=""
desktop_entry=""
while IFS= read -r installed_path; do
  case "$installed_path" in
    /usr/bin/*)
      binary_path="$installed_path"
      ;;
    /usr/share/applications/*.desktop)
      desktop_entry="${installed_path##*/}"
      desktop_entry="${desktop_entry%.desktop}"
      ;;
  esac
done < <(dpkg-query -L "$package_name")
test -n "$binary_path"
test -n "$desktop_entry"

export HOME="$xdg_root/home"
export XDG_CONFIG_HOME="$xdg_root/config"
export XDG_DATA_HOME="$xdg_root/data"
export XDG_STATE_HOME="$xdg_root/state"
mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME"

"$binary_path" --self-test | tee /tmp/notes-planner-package-self-test.log
grep -Fx 'PACKAGE_SELF_TEST_OK title_description_template_mapping_after_restart' \
  /tmp/notes-planner-package-self-test.log
test -f "$XDG_DATA_HOME/com.narguis.notes.desktop/notes-planner.sqlite3"

xvfb-run -a dbus-run-session -- sh -eu -c '
  gtk-launch "$1" &
  launcher_pid=$!
  trap "kill $launcher_pid 2>/dev/null || true" EXIT
  attempt=0
  while [ "$attempt" -lt 20 ]; do
    if xdotool search --name "Narguis Notes App" >/dev/null 2>&1; then
      exit 0
    fi
    attempt=$((attempt + 1))
    sleep 1
  done
  exit 1
' sh "$desktop_entry"

dpkg --purge "$package_name"
if dpkg-query -W --showformat='${db:Status-Abbrev}' "$package_name" 2>/dev/null; then
  printf 'Package still has a dpkg status after purge\n' >&2
  exit 1
fi
rm -rf "$xdg_root"
printf 'DEB_VERIFICATION_OK package=%s\n' "$package_name"
CONTAINER

printf 'DEB_VERIFICATION_OK artifacts=%s package=%s\n' "$artifact_directory" "$package_path"
