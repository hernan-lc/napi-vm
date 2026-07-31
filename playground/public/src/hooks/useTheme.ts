import { useCallback, useEffect, useState } from "preact/hooks";

const STORAGE_KEY = "napi-vm-theme";

export type Theme = "dark" | "light";

function detectTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "dark" || stored === "light") return stored;

  if (window.matchMedia?.("(prefers-color-scheme: light)").matches) return "light";
  return "dark";
}

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(detectTheme);

  const setTheme = useCallback((t: Theme) => {
    localStorage.setItem(STORAGE_KEY, t);
    setThemeState(t);
    document.documentElement.dataset.theme = t;
  }, []);

  const toggleTheme = useCallback(() => {
    setTheme(theme === "dark" ? "light" : "dark");
  }, [theme, setTheme]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, []);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      if (!localStorage.getItem(STORAGE_KEY)) {
        setThemeState(e.matches ? "dark" : "light");
        document.documentElement.dataset.theme = e.matches ? "dark" : "light";
      }
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  return { theme, setTheme, toggleTheme };
}
