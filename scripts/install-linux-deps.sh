#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Installs Debian packages on a CI runner: scripts/install-linux-deps.sh PACKAGE...
#
# apt sometimes hangs on a slow package mirror, which used to block a CI job
# for hours. Every apt command is time-limited and the install is retried.
set -uo pipefail

if [ "$#" -eq 0 ]; then
    echo "usage: $0 PACKAGE..." >&2
    exit 2
fi

for attempt in 1 2 3; do
    if sudo timeout 180 apt-get -o Acquire::Retries=3 update &&
        sudo timeout 600 apt-get -o Acquire::Retries=3 install -y --no-install-recommends "$@"; then
        exit 0
    fi
    echo "::warning::installing Linux packages failed or timed out (attempt $attempt of 3)"
    # An interrupted install leaves dpkg half-configured.
    sudo dpkg --configure -a || true
    sleep 10
done
exit 1
