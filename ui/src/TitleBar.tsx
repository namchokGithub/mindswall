import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import logo from "./assets/mindswall.png";

/** Minimal title bar drawn in the app's own colors. Dragging and double-click
 * maximize use Tauri's drag region; the buttons call the window API. */
export function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    const window = getCurrentWindow();
    let active = true; let dispose: (() => void) | undefined;
    const update = () => { window.isMaximized().then(value => { if (active) setMaximized(value); }).catch(() => {}); };
    update();
    window.onResized(update).then(unlisten => { if (active) dispose = unlisten; else unlisten(); }).catch(() => {});
    return () => { active = false; dispose?.(); };
  }, []);
  const window = () => getCurrentWindow();
  return <header className="titlebar" data-tauri-drag-region>
    <div className="titlebar-brand" data-tauri-drag-region><img src={logo} alt="" data-tauri-drag-region/><span data-tauri-drag-region>MindsWall</span></div>
    <div className="titlebar-controls">
      <button aria-label="Minimize" title="Minimize" onClick={() => window().minimize().catch(() => {})}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10"/></svg></button>
      <button aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"} onClick={() => window().toggleMaximize().catch(() => {})}>{maximized
        ? <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2.5 2.5V.5h7v7h-2M.5 2.5h7v7h-7z"/></svg>
        : <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M.5.5h9v9h-9z"/></svg>}</button>
      <button className="titlebar-close" aria-label="Close" title="Close" onClick={() => window().close().catch(() => {})}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10"/></svg></button>
    </div>
  </header>;
}
