import { useEffect, useState } from "react";

export type AppTheme = "light" | "dark";

function readTheme(): AppTheme {
  return document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light";
}

/** Reactively tracks the current theme (`data-theme` attribute on `<html>`). */
export function useTheme(): AppTheme {
  const [theme, setTheme] = useState<AppTheme>(readTheme);

  useEffect(() => {
    const observer = new MutationObserver(() => {
      setTheme(readTheme());
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    return () => observer.disconnect();
  }, []);

  return theme;
}
