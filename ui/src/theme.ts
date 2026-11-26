export type ThemeSetting = "system" | "light" | "dark";

/** The theme to draw: the saved choice, or the Windows app mode for "system". */
export function resolveTheme(setting: ThemeSetting, prefersDark: boolean): "light" | "dark" {
  return setting === "system" ? (prefersDark ? "dark" : "light") : setting;
}

/** Apply the theme to the page and follow Windows changes while set to "system". */
export function applyTheme(setting: ThemeSetting): () => void {
  const query = window.matchMedia("(prefers-color-scheme: dark)");
  const update = () => { document.documentElement.dataset.theme = resolveTheme(setting, query.matches); };
  update();
  if (setting !== "system") return () => {};
  query.addEventListener("change", update);
  return () => query.removeEventListener("change", update);
}
