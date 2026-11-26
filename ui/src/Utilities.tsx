import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Image, RefreshCw } from "lucide-react";
import { useSnackbar } from "./Snackbar";
import { createThumbnailQueue } from "./thumbnailQueue";

export type Filter = { min_width: number; min_height: number; orientation: "any" | "landscape" | "portrait" | "square"; aspect_ratio: "any" | "wide" | "wide_ten" | "standard" | "square" };
export type UtilityConfig = { excluded: string[]; filter: Filter; fit_mode: string };
type Page = { paths: string[]; total: number };
type Entry = { path: string; timestamp: number; monitor: string | null };

const loadThumbnail = createThumbnailQueue(4);

function Thumbnail({ path }: { path: string }) {
  const [src, setSrc] = useState("");
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true; let url = "";
    loadThumbnail(() => invoke<number[]>("gallery_thumbnail", { path }), () => active).then(bytes => {
      if (!active) return;
      url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: "image/png" })); setSrc(url);
    }).catch(() => { if (active) setFailed(true); });
    return () => { active = false; if (url) URL.revokeObjectURL(url); };
  }, [path]);
  return src ? <img src={src} alt="" loading="lazy" /> : <div className="thumbnail-placeholder">{failed ? "Preview unavailable" : "Loading…"}</div>;
}

export function Gallery({ config, busy, openSettings, exclude, restore }: { config: UtilityConfig; busy: boolean; openSettings: () => void; exclude: (path: string) => Promise<void>; restore: (path: string) => Promise<void> }) {
  const [paths, setPaths] = useState<string[]>([]); const [total, setTotal] = useState(0);
  const [view, setView] = useState<"collection" | "excluded">("collection");
  const [excludedLimit, setExcludedLimit] = useState(48);
  const [loading, setLoading] = useState(true); const [loadFailed, setLoadFailed] = useState(false); const { setError, setNotice } = useSnackbar("gallery");
  const load = useCallback(async (offset: number) => {
    setLoading(true); setLoadFailed(false); setError("");
    try { const page = await invoke<Page>("gallery_images", { offset }); setPaths(previous => offset === 0 ? page.paths : [...previous, ...page.paths]); setTotal(page.total); }
    catch (e) { setLoadFailed(true); setError(String(e)); } finally { setLoading(false); }
  }, [setError]);
  useEffect(() => { load(0); }, [load]);
  async function apply(path: string) { setError(""); try { await invoke("apply_image", { path }); setNotice("Wallpaper changed."); } catch (e) { setError(String(e)); } }
  const activeFilters = [config.filter.orientation !== "any" && `Orientation: ${config.filter.orientation}`, config.filter.aspect_ratio !== "any" && "Aspect ratio", (config.filter.min_width > 0 || config.filter.min_height > 0) && "Minimum resolution", config.excluded.length > 0 && `${config.excluded.length} excluded`].filter(Boolean);
  const visiblePaths = view === "excluded" ? config.excluded.slice(0, excludedLimit) : paths;
  const visibleTotal = view === "excluded" ? config.excluded.length : total;
  return <><div className="section-title"><div><span className="eyebrow">YOUR COLLECTION</span><h2>All wallpapers <span className="count-pill">{total}</span></h2></div><button className="icon-button" aria-label="Refresh gallery" disabled={loading} onClick={() => load(0)}><RefreshCw size={16}/></button></div>
    <div className="gallery-views" role="group" aria-label="Gallery view"><button className="secondary" aria-pressed={view === "collection"} disabled={busy} onClick={() => { if (view !== "collection") { setView("collection"); load(0); } }}>Collection</button><button className="secondary" aria-pressed={view === "excluded"} disabled={busy} onClick={() => { setView("excluded"); setExcludedLimit(48); }}>Excluded ({config.excluded.length})</button></div>
    {view === "excluded" && <p className="gallery-excluded-note">Restore an image to make it eligible for your collection and slideshow. Folder and image filters still apply.</p>}
    {view === "collection" && activeFilters.length > 0 && <div className="gallery-filter-note"><span>Active filters: {activeFilters.join(" · ")}</span><button className="text-link" onClick={openSettings}>Review settings</button></div>}
    {view === "collection" && loading && paths.length === 0 ? <div className="panel empty gallery-loading" role="status" aria-live="polite"><div className="gallery-spinner" aria-hidden="true"/><h3>Loading your collection…</h3><p>Finding wallpapers in your folders.</p></div> : view === "collection" && loadFailed && paths.length === 0 ? <div className="panel empty"><h3>Could not load your collection.</h3><p>Check your folders and try again.</p><button className="secondary" onClick={() => load(0)}>Try again</button></div> : visiblePaths.length === 0 ? <div className="panel empty"><div className="empty-icon"><Image size={25}/></div><h3>{view === "excluded" ? "No excluded images." : "No images found."}</h3><p>{view === "excluded" ? "Images you exclude will appear here." : "Add a folder or adjust your image filters."}</p></div> : <div className="gallery-grid" aria-busy={view === "collection" && loading}>{visiblePaths.map(path => <article className="gallery-card" key={path}><div className="gallery-thumb"><Thumbnail path={path}/></div><div className="gallery-body"><strong title={path}>{path.split(/[\\/]/).pop()}</strong><small className="gallery-source" title={path}>{path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")))}</small><div className="gallery-actions">{view === "collection" ? <><button disabled={busy || loading} onClick={() => apply(path)}>Set wallpaper</button><button disabled={busy || loading} onClick={async () => { try { await exclude(path); await load(0); } catch (e) { setError(String(e)); } }}>Exclude</button></> : <button disabled={busy} onClick={async () => { try { await restore(path); } catch (e) { setError(String(e)); } }}>Restore</button>}</div></div></article>)}</div>}
    {visiblePaths.length > 0 && <p className="gallery-count">Showing {visiblePaths.length} of {visibleTotal} wallpapers</p>}{view === "collection" && loading && paths.length > 0 && <p className="loading" role="status">Loading images…</p>}{visiblePaths.length < visibleTotal && <button className="secondary load-more" disabled={loading || busy} onClick={() => view === "excluded" ? setExcludedLimit(limit => limit + 48) : load(paths.length)}>Load more</button>}
  </>;
}

export function HistoryPage() {
  const [entries, setEntries] = useState<Entry[]>([]); const { setError, setNotice } = useSnackbar("history"); const [loading, setLoading] = useState(true);
  const refresh = useCallback(async () => { try { setEntries(await invoke<Entry[]>("get_history")); setError(""); } catch (e) { setError(String(e)); } finally { setLoading(false); } }, [setError]);
  useEffect(() => { refresh(); }, [refresh]);
  return <><div className="section-title"><div><span className="eyebrow">RECENT MOMENTS</span><h2>Wallpaper history</h2></div><button className="icon-button" aria-label="Refresh history" onClick={refresh}><RefreshCw size={16}/></button></div>
    {loading ? <p role="status">Loading history…</p> : entries.length === 0 ? <div className="panel empty"><h3>Nothing here yet.</h3><p>Wallpapers you apply will appear here.</p></div> : <div className="panel">{entries.map((entry, index) => <div className="history-row" key={`${entry.timestamp}-${index}`}><div><strong title={entry.path}>{entry.path.split(/[\\/]/).pop()}</strong><small title={entry.path}>{entry.path}</small><small>{new Date(entry.timestamp * 1000).toLocaleString()}{entry.monitor ? ` · ${entry.monitor}` : ""}</small></div><button className="secondary" onClick={async () => { try { await invoke("reapply_history", { path: entry.path, monitor: entry.monitor }); await refresh(); setNotice("Wallpaper changed."); } catch (e) { setError(String(e)); } }}>Apply again</button></div>)}</div>}
  </>;
}

type Monitor = { id: string; width: number; height: number };
export function MonitorsPage() {
  const [monitors, setMonitors] = useState<Monitor[]>([]); const { setError, setNotice } = useSnackbar("monitors"); const [loading, setLoading] = useState(true);
  const refresh = useCallback(async () => { setLoading(true); try { setMonitors(await invoke<Monitor[]>("monitors")); setError(""); } catch (e) { setError(String(e)); } finally { setLoading(false); } }, [setError]);
  useEffect(() => { refresh(); }, [refresh]);
  async function choose(monitor: Monitor) {
    try { const path = await open({ multiple: false, filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "bmp"] }] }); if (path) { await invoke("set_monitor_image", { monitor: monitor.id, path }); setNotice("Wallpaper changed for this monitor."); } }
    catch (e) { setError(String(e)); }
  }
  return <><div className="section-title"><div><span className="eyebrow">YOUR SCREENS</span><h2>Connected monitors</h2></div><button className="icon-button" aria-label="Refresh monitors" disabled={loading} onClick={refresh}><RefreshCw size={16}/></button></div>
    {loading ? <p role="status">Finding monitors…</p> : monitors.length === 0 ? <div className="panel empty"><h3>No monitors found.</h3><p>Connect a monitor, then refresh this page.</p></div> : <div className="panel">{monitors.map((monitor, index) => <div className="history-row" key={monitor.id}><div><strong>Monitor {index + 1}</strong><small>{monitor.width} × {monitor.height}</small><small title={monitor.id}>{monitor.id}</small></div><button className="secondary" onClick={() => choose(monitor)}>Choose image</button></div>)}</div>}
    <p className="footnote">A running slideshow can replace per-monitor images on its next change.</p>
  </>;
}
