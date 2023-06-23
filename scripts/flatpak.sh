#!/bin/bash
set -xe

scripts/generate-icons.sh

flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --noninteractive --user flathub org.freedesktop.Platform//22.08 org.freedesktop.Sdk//22.08 org.freedesktop.Sdk.Extension.rust-stable//22.08

flatpak install --noninteractive --user org.freedesktop.appstream-glib
flatpak run --env=G_DEBUG=fatal-criticals org.freedesktop.appstream-glib validate assets/halloy.appdata.xml

python3 -m pip install toml aiohttp
curl -L 'https://github.com/flatpak/flatpak-builder-tools/raw/master/cargo/flatpak-cargo-generator.py' > /tmp/flatpak-cargo-generator.py
python3 /tmp/flatpak-cargo-generator.py Cargo.lock -o assets/flatpak/generated-sources.json

if [ "${CI}" != "yes" ] ; then
  flatpak-builder \
    --install --force-clean --user -y \
    --state-dir /var/tmp/halloy-flatpak-builder \
    /var/tmp/halloy-flatpak-repo \
    assets/flatpak/org.squidowl.halloy.json
fi
