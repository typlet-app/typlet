# 0006. V1 platforms: Windows as product, Linux at developer quality

- Status: Accepted
- Date: 2026-10-23

## Context

Most of Typlet's target users work on Windows. The main developer works on Ubuntu 26.04, whose GNOME session is Wayland-only: global keyboard capture is deliberately not available to ordinary apps. A second platform from the start also keeps OS-specific assumptions out of the core.

## Decision

- **Windows 10 and 11** are the product target for V1: polished, signed, installable without admin rights.
- **Linux (Wayland)** is part of V1 at developer quality, using evdev for input and uinput for typing. Setup requires adding the user to the `input` group once.
- **macOS** follows after V1.

## Consequences

- The developer uses Typlet daily while building it.
- The platform layer is designed for several backends from day one.
- Linux setup is not yet suitable for non-technical users; a friendlier path (input method integration, GNOME extension) comes after V1.
- On GNOME, the focused app is not visible to Typlet without an extension, so per-app rules are limited on Linux in V1.
