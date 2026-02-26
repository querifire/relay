import type { Theme } from "./types";

export function getStoredTheme(): Theme {
  const stored = localStorage.getItem("autoproxy-theme");
  if (stored === "Light" || stored === "Dark") return stored;
  return "Dark";
}

export function applyTheme(theme: Theme) {
  localStorage.setItem("autoproxy-theme", theme);
  document.documentElement.setAttribute("data-theme", theme.toLowerCase());
}

export function initTheme() {
  applyTheme(getStoredTheme());
}
