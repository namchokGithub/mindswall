import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { RefreshButton } from "./RefreshButton";
import { LoadingBlock } from "./Spinner";
import { maskUserName } from "./maskPath";

/** The last 100 lines of the slideshow log, newest at the bottom. User names in paths are hidden. */
export function LogViewer({ close }: { close: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const body = useRef<HTMLPreElement>(null);
  const [lines, setLines] = useState<string[] | null>(null);
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    setError("");
    try { setLines(await invoke<string[]>("recent_log")); } catch (e) { setError(String(e)); }
  }, []);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    load();
    return () => { if (element?.open) element.close(); };
  }, [load]);
  // Show the newest lines first.
  useEffect(() => { if (body.current) body.current.scrollTop = body.current.scrollHeight; }, [lines]);
  return <dialog ref={dialog} className="gallery-dialog log-dialog" aria-labelledby="log-title" onCancel={event => { event.preventDefault(); close(); }}>
    <header className="gallery-dialog-header"><div><span className="eyebrow">LAST 100 LINES</span><h2 id="log-title">Recent log</h2></div><div className="profile-create"><RefreshButton label="Reload log" onRefresh={load}/><button autoFocus className="icon-button" aria-label="Close log" onClick={close}><X size={18}/></button></div></header>
    {error ? <div className="gallery-dialog-error" role="alert"><h3>Could not read the log.</h3><p>{error}</p></div>
      : lines === null ? <LoadingBlock label="Reading the log"/>
      : lines.length === 0 ? <p className="gallery-dialog-wait">The log is empty. It starts when the slideshow starts.</p>
      : <pre ref={body} className="log-lines selectable" tabIndex={0} aria-label="Log lines">{lines.map(maskUserName).join("\n")}</pre>}
  </dialog>;
}
