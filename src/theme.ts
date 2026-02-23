import type { Theme } from "./types";

/** Read persisted theme or fall back to Light (ProxyFlow design default). */
export function getStoredTheme(): Theme {
  const stored = localStorage.getItem("autoproxy-theme");
  if (stored === "Light" || stored === "Dark") return stored;
  return "Light";
}

/** Apply theme to `<html>` and persist in localStorage. */
export function applyTheme(theme: Theme) {
  localStorage.setItem("autoproxy-theme", theme);
  document.documentElement.setAttribute("data-theme", theme.toLowerCase());
}

/** Initialise theme on application start. */
export function initTheme() {
  applyTheme(getStoredTheme());
}
