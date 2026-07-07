import React, { createContext, useContext, useState } from "react";
import { en } from "./en";
import { pt } from "./pt";
import type { Strings } from "./en";

export type Locale = "en" | "pt";

const STORAGE_KEY = "claudia_locale";

const locales: Record<Locale, Strings> = { en, pt };

interface LocaleContextValue {
  locale: Locale;
  setLocale: (l: Locale) => void;
}

const LocaleContext = createContext<LocaleContextValue>({
  locale: "en",
  setLocale: () => {},
});

export const LocaleProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [locale, setLocaleState] = useState<Locale>(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored === "pt" ? "pt" : "en";
  });

  const setLocale = (l: Locale) => {
    localStorage.setItem(STORAGE_KEY, l);
    setLocaleState(l);
  };

  return React.createElement(
    LocaleContext.Provider,
    { value: { locale, setLocale } },
    children
  );
};

export const useLocale = () => useContext(LocaleContext);

export const useT = (): Strings => {
  const { locale } = useLocale();
  return locales[locale];
};
