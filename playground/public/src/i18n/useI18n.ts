import { useCallback, useEffect, useState } from "preact/hooks";
import { LOCALES, Locale, Translations } from "./translations.ts";

const STORAGE_KEY = "napi-vm-locale";

function detectLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && stored in LOCALES) return stored as Locale;

  const nav = navigator.language?.slice(0, 2);
  if (nav === "es" || nav === "pt") return nav;
  return "en";
}

export function useI18n() {
  const [locale, setLocaleState] = useState<Locale>(detectLocale);

  const setLocale = useCallback((l: Locale) => {
    localStorage.setItem(STORAGE_KEY, l);
    setLocaleState(l);
    document.documentElement.lang = l;
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, []);

  const t: Translations = LOCALES[locale];

  return { t, locale, setLocale };
}
