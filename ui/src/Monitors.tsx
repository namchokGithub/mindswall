import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ArrowLeft, ArrowRight, ArrowRightLeft, FolderOpen, Pencil, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useSnackbar } from "./Snackbar";
import { createThumbnailQueue } from "./thumbnailQueue";
import { Select } from "./Select";
import type { MonitorFolders, UtilityConfig } from "./Utilities";

export type Monitor = { id: string; name: string | null; x: number; y: number; width: number; height: number; primary: boolean; current: string | null };
export type MonitorName = { monitor: string; name: string };
/** The slideshow of a connected monitor with its own folders. */
export type MonitorPlayback = { monitor: string; current: string | null; position: number | null; total: number; combined?: string[] };
type FolderList = MonitorFolders["folders"];

const loadMonitorThumbnail = createThumbnailQueue(2);
const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

/** Monitors ordered left to right, then top to bottom, as numbered in the UI. */
export function sortMonitors(monitors: Monitor[]) {
  return [...monitors].sort((a, b) => a.x - b.x || a.y - b.y);
}

/** The hardware part of a device path, such as "DELD174", for monitors that are not connected. */
export function shortMonitorId(id: string) {
  return id.split("#")[1] || id;
}

/** The name the user gave a monitor, or its number among the connected monitors. */
export function monitorLabel(id: string, monitors: Monitor[], names: MonitorName[]) {
  const custom = names.find(entry => same(entry.monitor, id))?.name;
  if (custom) return custom;
  const index = monitors.findIndex(monitor => same(monitor.id, id));
  return index >= 0 ? `Monitor ${index + 1}` : `Monitor ${shortMonitorId(id)}`;
}

const hasOwnFolders = (config: UtilityConfig, id: string) =>
  config.monitor_folders.some(entry => same(entry.monitor, id) && entry.folders.some(folder => folder.enabled));

/** Load the connected monitors and reload them after each slideshow change. */
export function useMonitors(enabled = true) {
  const [monitors, setMonitors] = useState<Monitor[]>([]);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState("");
  const refresh = useCallback(async () => {
    setLoading(true);
    try { setMonitors(sortMonitors(await invoke<Monitor[]>("monitors"))); setError(""); }
    catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  }, []);
  useEffect(() => {
    if (!enabled) return;
    let active = true; let dispose: (() => void) | undefined;
    refresh();
    listen("slideshow-changed", () => { refresh(); }).then(unlisten => { if (active) dispose = unlisten; else unlisten(); }).catch(() => {});
    return () => { active = false; dispose?.(); };
  }, [enabled, refresh]);
  return { monitors, loading, error, refresh };
}

/** The image Windows shows on a monitor. Rust reads the path, so the UI only names the monitor. */
export function MonitorThumb({ monitor }: { monitor: Monitor }) {
  const [src, setSrc] = useState("");
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    setSrc(""); setFailed(false);
    if (!monitor.current) { setFailed(true); return; }
    let active = true; let url = "";
    loadMonitorThumbnail(() => invoke<ArrayBuffer>("monitor_thumbnail", { monitor: monitor.id }), () => active).then(bytes => {
      if (!active) return;
      url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: "image/png" })); setSrc(url);
    }).catch(() => { if (active) setFailed(true); });
    return () => { active = false; if (url) URL.revokeObjectURL(url); };
  }, [monitor.id, monitor.current]);
  return src ? <img src={src} alt=""/> : <div className="thumbnail-placeholder">{failed ? "No preview" : "Loading…"}</div>;
}

/** Monitor arrangement drawn to scale, like Windows display settings. With
 * `select`, each monitor is a button; otherwise `actions` adds controls that
 * appear when the pointer or focus is on a monitor. */
function MonitorLayout({ monitors, names, selected = null, select, detail, actions }: { monitors: Monitor[]; names: MonitorName[]; selected?: string | null; select?: (id: string) => void; detail?: (monitor: Monitor) => ReactNode; actions?: (monitor: Monitor) => ReactNode }) {
  const left = Math.min(...monitors.map(m => m.x)), top = Math.min(...monitors.map(m => m.y));
  const width = Math.max(...monitors.map(m => m.x + m.width)) - left;
  const height = Math.max(...monitors.map(m => m.y + m.height)) - top;
  return <div className="monitor-layout" style={{ aspectRatio: `${width} / ${height}`, width: `min(100%, 560px, ${Math.round(260 * width / height)}px)` }} role="group" aria-label="Monitor arrangement">
    {monitors.map((monitor, index) => {
      const style = { left: `${(monitor.x - left) / width * 100}%`, top: `${(monitor.y - top) / height * 100}%`, width: `${monitor.width / width * 100}%`, height: `${monitor.height / height * 100}%` };
      const label = <span className="monitor-tile-label"><strong>{index + 1}</strong>{monitorLabel(monitor.id, monitors, names)}{detail ? detail(monitor) : monitor.primary && <small>Primary</small>}</span>;
      return select
        ? <button key={monitor.id} type="button" className={`monitor-tile${selected === monitor.id ? " selected" : ""}`} aria-pressed={selected === monitor.id} title={monitor.name ?? undefined} onClick={() => select(monitor.id)} style={style}><MonitorThumb monitor={monitor}/>{label}</button>
        : <div key={monitor.id} className="monitor-tile" title={monitor.name ?? undefined} style={style}><MonitorThumb monitor={monitor}/>{actions && <div className="monitor-tile-actions">{actions(monitor)}</div>}{label}</div>;
    })}
  </div>;
}

/** Overview: the wallpaper on each monitor when some monitors run their own
 * slideshow, with Previous and Next for each of those monitors. */
export function MonitorStrip({ config, shows, stopped, busy, step }: { config: UtilityConfig; shows: MonitorPlayback[]; stopped: boolean; busy: boolean; step: (monitor: string, action: "next" | "previous") => void }) {
  const active = config.monitor_folders.some(entry => entry.folders.some(folder => folder.enabled));
  const { monitors } = useMonitors(active);
  if (!active || monitors.length === 0) return null;
  // Span stretches one image over every monitor, so single monitors cannot change.
  const span = config.fit_mode === "span";
  const showOf = (id: string) => shows.find(show => same(show.monitor, id));
  return <div className="overview-monitors">
    <MonitorLayout monitors={monitors} names={config.monitor_names}
      detail={monitor => { const show = showOf(monitor.id); return <small>{show ? `${show.position ?? "—"} / ${show.total}` : "Main"}</small>; }}
      actions={monitor => {
        const label = monitorLabel(monitor.id, monitors, config.monitor_names);
        if (span || stopped || !showOf(monitor.id)) return null;
        return <><button className="icon-button" disabled={busy} aria-label={`Previous wallpaper on ${label}`} title="Previous on this monitor" onClick={() => step(monitor.id, "previous")}><ArrowLeft size={15}/></button><button className="icon-button" disabled={busy} aria-label={`Next wallpaper on ${label}`} title="Next on this monitor" onClick={() => step(monitor.id, "next")}><ArrowRight size={15}/></button></>;
      }}/>
    <p className="footnote">{span ? "The Span fit mode shows one image across every monitor, so monitors cannot change on their own." : "Point at a monitor with its own folders to change only that monitor. The buttons below change every monitor."}</p>
  </div>;
}

export function MonitorsPage({ config, busy, saveMonitors }: { config: UtilityConfig; busy: boolean; saveMonitors: (folders: MonitorFolders[], names: MonitorName[]) => Promise<void> }) {
  const { monitors, loading, error, refresh } = useMonitors();
  const { setError, setNotice } = useSnackbar("monitors");
  const [selected, setSelected] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<{ id: string; value: string } | null>(null);
  const [moveTarget, setMoveTarget] = useState<Record<string, string>>({});
  const [counts, setCounts] = useState<Record<string, number | null>>({});
  const cards = useRef<Record<string, HTMLDivElement | null>>({});
  useEffect(() => { if (error) setError(error); }, [error, setError]);

  const foldersOf = (id: string) => config.monitor_folders.find(entry => same(entry.monitor, id))?.folders ?? [];
  const nameOf = (id: string) => config.monitor_names.find(entry => same(entry.monitor, id))?.name ?? "";
  // Entries without folders or names are dropped so a monitor returns to the main slideshow.
  const withFolders = (id: string, folders: FolderList) => [...config.monitor_folders.filter(entry => !same(entry.monitor, id)), ...(folders.length > 0 ? [{ monitor: id, folders }] : [])];
  const withName = (names: MonitorName[], id: string, name: string) => [...names.filter(entry => !same(entry.monitor, id)), ...(name ? [{ monitor: id, name }] : [])];

  // Recount only when the monitor folders change, not on every config save.
  const ownFolderKey = config.monitor_folders.map(entry => entry.folders.map(folder => `${folder.path}|${folder.recursive}`).join(",")).join(";");
  useEffect(() => {
    let active = true;
    const paths = config.monitor_folders.flatMap(entry => entry.folders);
    Promise.all(paths.map(async folder => {
      try { return [folder.path, await invoke<number>("folder_count", { path: folder.path, recursive: folder.recursive })] as const; }
      catch { return [folder.path, null] as const; }
    })).then(entries => { if (active) setCounts(Object.fromEntries(entries)); });
    return () => { active = false; };
  }, [ownFolderKey]);

  async function run(action: () => Promise<void>) { try { await action(); } catch (e) { setError(String(e)); } }
  const addFolder = (id: string) => run(async () => {
    const path = await invoke<string | null>("pick_folder");
    if (!path || foldersOf(id).some(folder => same(folder.path, path))) return;
    await saveMonitors(withFolders(id, [...foldersOf(id), { path, enabled: true, recursive: true }]), config.monitor_names);
  });
  const removeFolder = (id: string, path: string) => run(() => saveMonitors(withFolders(id, foldersOf(id).filter(folder => folder.path !== path)), config.monitor_names));
  const chooseImage = (id: string) => run(async () => {
    const path = await invoke<string | null>("pick_image");
    if (!path) return;
    await invoke("set_monitor_image", { monitor: id, path }); setNotice("Wallpaper changed for this monitor."); await refresh();
  });
  const saveName = () => run(async () => {
    if (!renaming) return;
    const name = renaming.value.trim();
    if ([...name].length > 40) { setError("Use 40 characters or fewer."); return; }
    await saveMonitors(config.monitor_folders, withName(config.monitor_names, renaming.id, name));
    setRenaming(null);
  });
  const forget = (id: string) => run(() => saveMonitors(withFolders(id, []), withName(config.monitor_names, id, "")));
  const move = (from: string) => run(async () => {
    const to = moveTarget[from];
    if (!to) return;
    let names = withName(config.monitor_names, from, "");
    if (nameOf(from) && !nameOf(to)) names = withName(names, to, nameOf(from));
    const folders = withFolders(from, []).filter(entry => !same(entry.monitor, to));
    await saveMonitors([...folders, ...(foldersOf(from).length > 0 ? [{ monitor: to, folders: foldersOf(from) }] : [])], names);
    setNotice("Monitor settings moved.");
  });
  function select(id: string) { setSelected(id); cards.current[id]?.scrollIntoView({ behavior: "smooth", block: "nearest" }); }

  const imageCount = (folders: FolderList) => {
    const values = folders.filter(folder => folder.enabled).map(folder => counts[folder.path]);
    return values.some(value => value === undefined || value === null) ? null : values.reduce<number>((sum, value) => sum + (value ?? 0), 0);
  };
  const mode = (id: string) => {
    const folders = foldersOf(id);
    if (folders.length === 0) return "Main slideshow";
    const total = imageCount(folders);
    return `Own folders · ${folders.length} ${folders.length === 1 ? "folder" : "folders"}${total === null ? "" : ` · ${total} images`}`;
  };
  const folderList = (id: string) => foldersOf(id).map(folder => <div className="monitor-folder" key={folder.path}><FolderOpen size={15}/><small title={folder.path}>{folder.path}</small><button className="icon-button danger" aria-label={`Remove ${folder.path}`} disabled={busy} onClick={() => removeFolder(id, folder.path)}><Trash2 size={14}/></button></div>);

  const savedIds = useMemo(() => {
    const ids = [...config.monitor_folders.map(entry => entry.monitor), ...config.monitor_names.map(entry => entry.monitor)];
    return ids.filter((id, index) => ids.findIndex(other => same(other, id)) === index && !monitors.some(monitor => same(monitor.id, id)));
  }, [config.monitor_folders, config.monitor_names, monitors]);
  const moveTargets = monitors.filter(monitor => foldersOf(monitor.id).length === 0);

  return <><div className="section-title"><div><span className="eyebrow">YOUR SCREENS</span><h2>Connected monitors</h2></div><button className="icon-button" aria-label="Refresh monitors" disabled={loading} onClick={refresh}><RefreshCw size={16}/></button></div>
    {loading && monitors.length === 0 ? <p role="status">Finding monitors…</p> : error && monitors.length === 0 ? <div className="panel empty"><h3>Could not load monitors.</h3><p>Try refreshing the list.</p><button className="secondary" onClick={refresh}>Try again</button></div> : monitors.length === 0 ? <div className="panel empty"><h3>No monitors found.</h3><p>Connect a monitor, then refresh this page.</p></div> : <>
      <MonitorLayout monitors={monitors} names={config.monitor_names} selected={selected} select={select}/>
      <div className="panel">{monitors.map((monitor, index) => <div className={`history-row monitor-card${selected === monitor.id ? " selected" : ""}`} key={monitor.id} ref={element => { cards.current[monitor.id] = element; }}>
        <div className="history-thumbnail monitor-thumbnail"><MonitorThumb monitor={monitor}/></div>
        <div className="monitor-info">
          {renaming?.id === monitor.id
            ? <form className="monitor-rename" onSubmit={event => { event.preventDefault(); saveName(); }}><input aria-label="Monitor name" autoFocus maxLength={40} placeholder={`Monitor ${index + 1}`} value={renaming.value} onChange={event => setRenaming({ id: monitor.id, value: event.target.value })} onKeyDown={event => { if (event.key === "Escape") setRenaming(null); }}/><button className="secondary" type="submit" disabled={busy}>Save</button><button className="secondary" type="button" onClick={() => setRenaming(null)}>Cancel</button></form>
            : <div className="monitor-title"><strong>{monitorLabel(monitor.id, monitors, config.monitor_names)}</strong><button className="icon-button" aria-label="Rename monitor" title="Rename" disabled={busy} onClick={() => setRenaming({ id: monitor.id, value: nameOf(monitor.id) })}><Pencil size={13}/></button></div>}
          <small title={monitor.id}>{[nameOf(monitor.id) && `Monitor ${index + 1}`, monitor.name ?? "Unknown model", `${monitor.width} × ${monitor.height}`, monitor.primary && "Primary"].filter(Boolean).join(" · ")}</small>
          <span className={`monitor-mode${foldersOf(monitor.id).length > 0 ? " own" : ""}`}>{mode(monitor.id)}</span>
          <small className="selectable" title={monitor.current ?? undefined}>{!monitor.current ? "No wallpaper image reported" : /[\\/]collages[\\/][^\\/]+$/i.test(monitor.current) ? "Showing combined portrait images" : `Showing ${monitor.current.split(/[\\/]/).pop()}`}</small>
          {folderList(monitor.id)}
        </div>
        <div className="monitor-actions"><button className="secondary" disabled={busy} onClick={() => addFolder(monitor.id)}><Plus size={15}/> Add folder</button><button className="secondary" onClick={() => chooseImage(monitor.id)}>Choose image</button></div>
      </div>)}</div></>}
    {savedIds.length > 0 && <><div className="section-title"><div><span className="eyebrow">NOT CONNECTED</span><h2>Saved monitor settings</h2></div></div><div className="panel">{savedIds.map(id => <div className="history-row" key={id}><div className="monitor-info"><strong title={id}>{monitorLabel(id, monitors, config.monitor_names)}</strong><small title={id}>{nameOf(id) ? shortMonitorId(id) : "Not connected"}</small>{foldersOf(id).length === 0 ? <small>No folders saved.</small> : folderList(id)}</div>
      <div className="monitor-actions">{moveTargets.length > 0 && <div className="monitor-move"><Select label="Move settings to monitor" placeholder="Move to…" value={moveTarget[id] ?? null} disabled={busy} onChange={target => setMoveTarget(targets => ({ ...targets, [id]: target }))} options={moveTargets.map(monitor => ({ value: monitor.id, label: monitorLabel(monitor.id, monitors, config.monitor_names) }))}/><button className="secondary" aria-label="Move settings" disabled={busy || !moveTarget[id]} onClick={() => move(id)}><ArrowRightLeft size={14}/> Move</button></div>}<button className="secondary" disabled={busy} onClick={() => forget(id)}>Forget</button></div></div>)}</div>
      <p className="footnote">A monitor can get a new device path when it is moved to another port. Move its settings to the monitor's new entry.</p></>}
    <p className="footnote">A monitor with its own folders runs its own slideshow; other monitors share the main slideshow. Interval, shuffle, filters, and exclusions are shared by every monitor, and Windows uses one fit mode for the whole desktop. Connected and disconnected monitors are noticed within about 10 seconds. A single image chosen here lasts until the next change.</p>
  </>;
}
