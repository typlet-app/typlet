// SPDX-License-Identifier: GPL-3.0-or-later
/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "1" builds the demo mode with sample data instead of the Rust backend. */
  readonly VITE_DEMO?: string;
}
