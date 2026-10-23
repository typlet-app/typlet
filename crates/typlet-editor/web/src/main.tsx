// SPDX-License-Identifier: GPL-3.0-or-later
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";

// Demo mode for screenshots and interface work without Tauri.
if (import.meta.env.VITE_DEMO === "1") {
  await import("./demo");
}

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
