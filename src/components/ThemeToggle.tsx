import { useState } from "react";
import { getStoredTheme, applyTheme } from "../theme";
import type { Theme } from "../types";

export default function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(getStoredTheme);

  const toggle = () => {
    const next: Theme = theme === "Dark" ? "Light" : "Dark";
    setTheme(next);
    applyTheme(next);
  };

  return (
    <button
      onClick={toggle}
      className="flex items-center gap-3 w-full px-3 py-[0.625rem] rounded-button text-[0.875rem] font-medium text-foreground-muted hover:text-foreground hover:bg-surface-hover transition-all duration-200"
      title={`Switch to ${theme === "Dark" ? "light" : "dark"} mode`}
    >
      {theme === "Dark" ? (
        <svg
          className="w-[1.125rem] h-[1.125rem]"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="5" />
          <line x1="12" y1="1" x2="12" y2="3" />
          <line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" />
          <line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </svg>
      ) : (
        <svg
          className="w-[1.125rem] h-[1.125rem]"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      )}
      {theme === "Dark" ? "Light mode" : "Dark mode"}
    </button>
  );
}
