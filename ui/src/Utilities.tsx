import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Grid2X2, Grid3X3, Image, List, RefreshCw } from "lucide-react";
import { useSnackbar } from "./Snackbar";
import { createThumbnailQueue } from "./thumbnailQueue";
import { GalleryPreview } from "./GalleryPreview";
import { monitorLabel, useMonitors, type MonitorName } from "./Monitors";

export type Filter = { min_width: number; min_height: number; orientation: "any" | "landscape" | "portrait" | "square"; aspect_ratio: "any" | "wide" | "wide_ten" | "standard" | "square" };
export type MonitorFolders = { monitor: string; folders: { path: string; enabled: boolean; recursive: boolean }[] };
export type UtilityConfig = { excluded: string[]; filter: Filter; fit_mode: string; monitor_folders: MonitorFolders[]; monitor_names: MonitorName[] };
type Page = { paths: string[]; total: number };
type Entry = { path: string; timestamp: number; monitor: string | null };
type GalleryLayout = "large" | "compact" | "list";

function initialGalleryLayout(): GalleryLayout {
  try {
    const saved = localStorage.getItem("mindswall.gallery.layout");
    return saved === "compact" || saved === "list" ? saved : "large";
  } catch { return "large"; }
}

const loadThumbnail = createThumbnailQueue(4);

function Thumbnail({ path, source = "gallery_thumbnail" }: { path: string; source?: "gallery_thumbnail" | "history_thumbnail" }) {
  const [src, setSrc] = useState("");
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true; let url = "";
    loadThumbnail(() => invoke<ArrayBuffer>(source, { path }), () => active).then(bytes => {
      if (!active) return;
      url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: "image/png" })); setSrc(url);
    }).catch(() => { if (active) setFailed(true); });
    return () => { active = false; if (url) URL.revokeObjectURL(url); };
  }, [path, source]);
  return src ? <img src={src} alt="" loading="lazy" /> : <div className="thumbnail-placeholder">{failed ? "Preview unavailable" : "Loading…"}</div>;
}

export function Gallery({ config, busy, collectionVersion, openSettings, exclude, restore }: { config: UtilityConfig; busy: boolean; collectionVersion: number; openSettings: () => void; exclude: (path: string) => Promise<void>; restore: (path: string) => Promise<void> }) {
  const [paths, setPaths] = useState<string[]>([]); const [total, setTotal] = useState(0);
  const [view, setView] = useState<"collection" | "excluded">("collection");
  const [layout, setLayout] = useState<GalleryLayout>(initialGalleryLayout);
  const [selected, setSelected] = useState<string | null>(null);
  const [excludedLimit, setExcludedLimit] = useState(48);
  const [loading, setLoading] = useState(true); const [loadFailed, setLoadFailed] = useState(false); const { setError, setNotice } = useSnackbar("gallery");
  const load = useCallback(async (offset: number) => {
    setLoading(true); setLoadFailed(false); setError("");
    try { const page = await invoke<Page>("gallery_images", { offset }); setPaths(previous => offset === 0 ? page.paths : [...previous, ...page.paths]); setTotal(page.total); }
    catch (e) { setLoadFailed(true); setError(String(e)); } finally { setLoading(false); }
  }, [setError]);
  useEffect(() => { load(0); }, [load]);
  // Reload the first page when the folder watcher changes the collection.
  const shownVersion = useRef(collectionVersion);
  useEffect(() => {
    if (shownVersion.current === collectionVersion) return;
    shownVersion.current = collectionVersion;
    if (view === "collection") load(0);
  }, [collectionVersion, view, load]);
  async function apply(path: string) { setError(""); try { await invoke("apply_image", { path }); setNotice("Wallpaper changed."); } catch (e) { setError(String(e)); } }
  const activeFilters = [config.filter.orientation !== "any" && `Orientation: ${config.filter.orientation}`, config.filter.aspect_ratio !== "any" && "Aspect ratio", (config.filter.min_width > 0 || config.filter.min_height > 0) && "Minimum resolution", config.excluded.length > 0 && `${config.excluded.length} excluded`].filter(Boolean);
  const visiblePaths = view === "excluded" ? config.excluded.slice(0, excludedLimit) : paths;
  const visibleTotal = view === "excluded" ? config.excluded.length : total;
  function changeLayout(next: GalleryLayout) {
    setLayout(next);
    try { localStorage.setItem("mindswall.gallery.layout", next); } catch { /* Keep the current choice for this session. */ }
  }
  return <>{selected && <GalleryPreview key={selected} path={selected} close={() => setSelected(null)}/>}<div className="section-title"><div><span className="eyebrow">YOUR COLLECTION</span><h2>All wallpapers <span className="count-pill">{total}</span></h2></div><button className="icon-button" aria-label="Refresh gallery" disabled={loading} onClick={() => load(0)}><RefreshCw size={16}/></button></div>
    <div className="gallery-toolbar"><div className="gallery-views" role="group" aria-label="Gallery collection"><button className="secondary" aria-pressed={view === "collection"} disabled={busy} onClick={() => { if (view !== "collection") { setView("collection"); load(0); } }}>Collection</button><button className="secondary" aria-pressed={view === "excluded"} disabled={busy} onClick={() => { setView("excluded"); setExcludedLimit(48); }}>Excluded ({config.excluded.length})</button></div><div className="gallery-layouts" role="group" aria-label="Gallery layout"><button className="icon-button" aria-label="Large grid" title="Large grid" aria-pressed={layout === "large"} onClick={() => changeLayout("large")}><Grid2X2 size={16}/></button><button className="icon-button" aria-label="Compact grid" title="Compact grid" aria-pressed={layout === "compact"} onClick={() => changeLayout("compact")}><Grid3X3 size={16}/></button><button className="icon-button" aria-label="List" title="List" aria-pressed={layout === "list"} onClick={() => changeLayout("list")}><List size={16}/></button></div></div>
    {view === "excluded" && <p className="gallery-excluded-note">Restore an image to make it eligible for your collection and slideshow. Folder and image filters still apply.</p>}
    {view === "collection" && activeFilters.length > 0 && <div className="gallery-filter-note"><span>Active filters: {activeFilters.join(" · ")}</span><button className="text-link" onClick={openSettings}>Review settings</button></div>}
    {view === "collection" && loading && paths.length === 0 ? <div className="panel empty gallery-loading" role="status" aria-live="polite"><div className="gallery-spinner" aria-hidden="true"/><h3>Loading your collection…</h3><p>Finding wallpapers in your folders.</p></div> : view === "collection" && loadFailed && paths.length === 0 ? <div className="panel empty"><h3>Could not load your collection.</h3><p>Check your folders and try again.</p><button className="secondary" onClick={() => load(0)}>Try again</button></div> : visiblePaths.length === 0 ? <div className="panel empty"><div className="empty-icon"><Image size={25}/></div><h3>{view === "excluded" ? "No excluded images." : "No images found."}</h3><p>{view === "excluded" ? "Images you exclude will appear here." : "Add a folder or adjust your image filters."}</p></div> : <div className={`gallery-grid gallery-grid--${layout}`} aria-busy={view === "collection" && loading}>{visiblePaths.map(path => <article className="gallery-card" key={path}><button className="gallery-thumb gallery-preview-button" aria-label={`Preview ${path.split(/[\\/]/).pop()}`} onClick={() => setSelected(path)}><Thumbnail path={path}/></button><div className="gallery-body"><strong title={path.split(/[\\/]/).pop()}>{path.split(/[\\/]/).pop()}</strong><div className="gallery-actions">{view === "collection" ? <><button disabled={busy || loading} onClick={() => apply(path)}>Set wallpaper</button><button disabled={busy || loading} onClick={async () => { try { await exclude(path); await load(0); } catch (e) { setError(String(e)); } }}>Exclude</button></> : <button disabled={busy} onClick={async () => { try { await restore(path); } catch (e) { setError(String(e)); } }}>Restore</button>}</div></div></article>)}</div>}
    {visiblePaths.length > 0 && <p className="gallery-count">Showing {visiblePaths.length} of {visibleTotal} wallpapers</p>}{view === "collection" && loading && paths.length > 0 && <p className="loading" role="status">Loading images…</p>}{visiblePaths.length < visibleTotal && <button className="secondary load-more" disabled={loading || busy} onClick={() => view === "excluded" ? setExcludedLimit(limit => limit + 48) : load(paths.length)}>Load more</button>}
  </>;
}

export function HistoryPage({ names }: { names: MonitorName[] }) {
  const [entries, setEntries] = useState<Entry[]>([]); const { setError, setNotice } = useSnackbar("history"); const [loading, setLoading] = useState(true);
  // "" shows every entry, "main" the main slideshow, any other value one monitor.
  const [filter, setFilter] = useState("");
  const { monitors } = useMonitors();
  const refresh = useCallback(async () => { try { setEntries(await invoke<Entry[]>("get_history")); setError(""); } catch (e) { setError(String(e)); } finally { setLoading(false); } }, [setError]);
  useEffect(() => { refresh(); }, [refresh]);
  const label = (monitor: string | null) => monitor ? monitorLabel(monitor, monitors, names) : "Main slideshow";
  const targets = [...new Set(entries.map(entry => entry.monitor).filter((monitor): monitor is string => monitor !== null))];
  const shown = entries.filter(entry => filter === "" || (filter === "main" ? entry.monitor === null : entry.monitor === filter));
  const hasMonitors = targets.length > 0;
  return <><div className="section-title"><div><span className="eyebrow">RECENT MOMENTS</span><h2>Wallpaper history</h2></div><div className="history-tools">{hasMonitors && <select aria-label="Show history for" value={filter} onChange={event => setFilter(event.target.value)}><option value="">All monitors</option><option value="main">Main slideshow</option>{targets.map(monitor => <option key={monitor} value={monitor}>{label(monitor)}</option>)}</select>}<button className="icon-button" aria-label="Refresh history" onClick={refresh}><RefreshCw size={16}/></button></div></div>
    {loading ? <p role="status">Loading history…</p> : shown.length === 0 ? <div className="panel empty"><h3>Nothing here yet.</h3><p>{entries.length === 0 ? "Wallpapers you apply will appear here." : "No wallpapers for this monitor in the latest 10 changes."}</p></div> : <div className="panel">{shown.map((entry, index) => <div className="history-row history-entry" key={`${entry.timestamp}-${index}`}><div className="history-thumbnail"><Thumbnail path={entry.path} source="history_thumbnail"/></div><div className="history-details"><strong title={entry.path}>{entry.path.split(/[\\/]/).pop()}</strong><small title={entry.path}>{entry.path}</small><small title={entry.monitor ?? undefined}>{new Date(entry.timestamp * 1000).toLocaleString()}{hasMonitors ? ` · ${label(entry.monitor)}` : ""}</small></div><button className="secondary" onClick={async () => { try { await invoke("reapply_history", { path: entry.path, monitor: entry.monitor }); await refresh(); setNotice("Wallpaper changed."); } catch (e) { setError(String(e)); } }}>Apply again</button></div>)}</div>}
    {hasMonitors && <p className="footnote">History keeps the latest 10 changes across all monitors.</p>}
  </>;
}

