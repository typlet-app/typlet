#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Runs after the Debian package is installed (as root).
#
# Typlet reads keyboards (group "input") and types through a virtual
# keyboard (/dev/uinput, opened to that group by the udev rule this
# package installs).
set -e

if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger --subsystem-match=misc --sysname-match=uinput || true
fi

# When installed with sudo, give the installing user keyboard access.
if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ] && command -v usermod >/dev/null 2>&1; then
    if usermod -aG input "$SUDO_USER"; then
        echo "Typlet: added $SUDO_USER to the group 'input'. Log out and back in to use Typlet."
    fi
else
    echo "Typlet: run 'sudo usermod -aG input \$USER', then log out and back in."
fi

exit 0
