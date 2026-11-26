import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FolderOpen, Grid2X2, Grid3X3, Image, List, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useSnackbar } from "./Snackbar";
import { createThumbnailQueue } from "./thumbnailQueue";
import { GalleryPreview } from "./GalleryPreview";

export type Filter = { min_width: number; min_height: number; orientation: "any" | "landscape" | "portrait" | "square"; aspect_ratio: "any" | "wide" | "wide_ten" | "standard" | "square" };
export type MonitorFolders = { monitor: string; folders: { path: string; enabled: boolean; recursive: boolean }[] };
export type UtilityConfig = { excluded: string[]; filter: Filter; fit_mode: string; monitor_folders: MonitorFolders[] };
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

export function Gallery({ config, busy, openSettings, exclude, restore }: { config: UtilityConfig; busy: boolean; openSettings: () => void; exclude: (path: string) => Promise<void>; restore: (path: string) => Promise<void> }) {
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

export function HistoryPage() {
  const [entries, setEntries] = useState<Entry[]>([]); const { setError, setNotice } = useSnackbar("history"); const [loading, setLoading] = useState(true);
  const refresh = useCallback(async () => { try { setEntries(await invoke<Entry[]>("get_history")); setError(""); } catch (e) { setError(String(e)); } finally { setLoading(false); } }, [setError]);
  useEffect(() => { refresh(); }, [refresh]);
  return <><div className="section-title"><div><span className="eyebrow">RECENT MOMENTS</span><h2>Wallpaper history</h2></div><button className="icon-button" aria-label="Refresh history" onClick={refresh}><RefreshCw size={16}/></button></div>
    {loading ? <p role="status">Loading history…</p> : entries.length === 0 ? <div className="panel empty"><h3>Nothing here yet.</h3><p>Wallpapers you apply will appear here.</p></div> : <div className="panel">{entries.map((entry, index) => <div className="history-row history-entry" key={`${entry.timestamp}-${index}`}><div className="history-thumbnail"><Thumbnail path={entry.path} source="history_thumbnail"/></div><div className="history-details"><strong title={entry.path}>{entry.path.split(/[\\/]/).pop()}</strong><small title={entry.path}>{entry.path}</small><small>{new Date(entry.timestamp * 1000).toLocaleString()}{entry.monitor ? ` · ${entry.monitor}` : ""}</small></div><button className="secondary" onClick={async () => { try { await invoke("reapply_history", { path: entry.path, monitor: entry.monitor }); await refresh(); setNotice("Wallpaper changed."); } catch (e) { setError(String(e)); } }}>Apply again</button></div>)}</div>}
  </>;
}

type Monitor = { id: string; width: number; height: number };
export function MonitorsPage({ config, busy, saveMonitorFolders }: { config: UtilityConfig; busy: boolean; saveMonitorFolders: (value: MonitorFolders[]) => Promise<void> }) {
  const [monitors, setMonitors] = useState<Monitor[]>([]); const { setError, setNotice } = useSnackbar("monitors"); const [loading, setLoading] = useState(true); const [loadFailed, setLoadFailed] = useState(false);
  const refresh = useCallback(async () => { setLoading(true); setLoadFailed(false); try { setMonitors(await invoke<Monitor[]>("monitors")); setError(""); } catch (e) { setLoadFailed(true); setError(String(e)); } finally { setLoading(false); } }, [setError]);
  useEffect(() => { refresh(); }, [refresh]);
  async function choose(monitor: Monitor) {
    try { const path = await invoke<string | null>("pick_image"); if (path) { await invoke("set_monitor_image", { monitor: monitor.id, path }); setNotice("Wallpaper changed for this monitor."); } }
    catch (e) { setError(String(e)); }
  }
  const foldersOf = (id: string) => config.monitor_folders.find(entry => entry.monitor === id)?.folders ?? [];
  // Entries without folders are dropped so a monitor returns to the main slideshow.
  const withFolders = (id: string, folders: MonitorFolders["folders"]) => [...config.monitor_folders.filter(entry => entry.monitor !== id), ...(folders.length > 0 ? [{ monitor: id, folders }] : [])];
  async function addFolder(id: string) {
    try {
      const path = await invoke<string | null>("pick_folder");
      if (!path || foldersOf(id).some(folder => folder.path.toLowerCase() === path.toLowerCase())) return;
      await saveMonitorFolders(withFolders(id, [...foldersOf(id), { path, enabled: true, recursive: true }]));
    } catch (e) { setError(String(e)); }
  }
  const removeFolder = (id: string, path: string) => saveMonitorFolders(withFolders(id, foldersOf(id).filter(folder => folder.path !== path)));
  const folderList = (id: string) => foldersOf(id).length === 0 ? <small>Shows the main slideshow.</small> : foldersOf(id).map(folder => <div className="monitor-folder" key={folder.path}><FolderOpen size={15}/><small title={folder.path}>{folder.path}</small><button className="icon-button danger" aria-label={`Remove ${folder.path}`} disabled={busy} onClick={() => removeFolder(id, folder.path)}><Trash2 size={14}/></button></div>);
  const disconnected = config.monitor_folders.filter(entry => !monitors.some(monitor => monitor.id === entry.monitor));
  return <><div className="section-title"><div><span className="eyebrow">YOUR SCREENS</span><h2>Connected monitors</h2></div><button className="icon-button" aria-label="Refresh monitors" disabled={loading} onClick={refresh}><RefreshCw size={16}/></button></div>
    {loading ? <p role="status">Finding monitors…</p> : loadFailed ? <div className="panel empty"><h3>Could not load monitors.</h3><p>Try refreshing the list.</p><button className="secondary" onClick={refresh}>Try again</button></div> : monitors.length === 0 ? <div className="panel empty"><h3>No monitors found.</h3><p>Connect a monitor, then refresh this page.</p></div> : <div className="panel">{monitors.map((monitor, index) => <div className="history-row" key={monitor.id}><div><strong>Monitor {index + 1}</strong><small>{monitor.width} × {monitor.height}</small><small title={monitor.id}>{monitor.id}</small>{folderList(monitor.id)}</div><div className="monitor-actions"><button className="secondary" disabled={busy} onClick={() => addFolder(monitor.id)}><Plus size={15}/> Add folder</button><button className="secondary" onClick={() => choose(monitor)}>Choose image</button></div></div>)}</div>}
    {!loading && !loadFailed && disconnected.length > 0 && <><div className="section-title"><div><span className="eyebrow">NOT CONNECTED</span><h2>Saved monitor folders</h2></div></div><div className="panel">{disconnected.map(entry => <div className="history-row" key={entry.monitor}><div><small title={entry.monitor}>{entry.monitor}</small>{folderList(entry.monitor)}</div><button className="secondary" disabled={busy} onClick={() => saveMonitorFolders(withFolders(entry.monitor, []))}>Forget</button></div>)}</div></>}
    <p className="footnote">A monitor with its own folders runs its own slideshow at the same interval; other monitors share the main slideshow. Connected and disconnected monitors are noticed within about 10 seconds. A single image chosen here lasts until the next change.</p>
  </>;
}
