import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Minimize, maximize, and close buttons floating over the top-right corner of
 * the window, in the style of Windows 11 caption buttons. There is no separate
 * title bar: the sidebar and the top line of the page are the drag regions, and
 * double-clicking them maximizes and restores. The buttons dim while the window
 * is not focused, as Windows does. */
export function WindowControls() {
  const [maximized, setMaximized] = useState(false);
  const [focused, setFocused] = useState(true);
  useEffect(() => {
    const window = getCurrentWindow();
    let active = true; const disposers: (() => void)[] = [];
    const keep = (unlisten: () => void) => { if (active) disposers.push(unlisten); else unlisten(); };
    const update = () => { window.isMaximized().then(value => { if (active) setMaximized(value); }).catch(() => {}); };
    update();
    window.onResized(update).then(keep).catch(() => {});
    window.onFocusChanged(({ payload }) => { if (active) setFocused(payload); }).then(keep).catch(() => {});
    return () => { active = false; disposers.forEach(dispose => dispose()); };
  }, []);
  const window = () => getCurrentWindow();
  return <div className={`window-controls${focused ? "" : " is-inactive"}`}>
    <button aria-label="Minimize" title="Minimize" onClick={() => window().minimize().catch(() => {})}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10"/></svg></button>
    <button aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"} onClick={() => window().toggleMaximize().catch(() => {})}>{maximized
      ? <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2.5 2.5V.5h7v7h-2M.5 2.5h7v7h-7z"/></svg>
      : <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M.5.5h9v9h-9z"/></svg>}</button>
    <button className="window-close" aria-label="Close" title="Close" onClick={() => window().close().catch(() => {})}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10"/></svg></button>
  </div>;
}
