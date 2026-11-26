import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ArrowRightLeft, Copy, ExternalLink, FolderOpen, Pencil, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useSnackbar } from "./Snackbar";
import { createThumbnailQueue } from "./thumbnailQueue";
import { useLoadedImage } from "./useLoadedImage";
import { Select } from "./Select";
import { PathCopy } from "./PathCopy";
import { maskFolderPath } from "./maskPath";
import { ConfirmDialog, type Confirmation } from "./ConfirmDialog";
import type { MonitorFolders, UtilityConfig } from "./Utilities";

export type Monitor = { id: string; name: string | null; x: number; y: number; width: number; height: number; primary: boolean; current: string | null };
export type MonitorName = { monitor: string; name: string };
/** The slideshow of a connected monitor with its own folders. */
export type MonitorPlayback = { monitor: string; current: string | null; position: number | null; total: number; combined?: string[]; display?: string | null };
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
  const { src, loading, failed } = useLoadedImage(`${monitor.id}|${monitor.current ?? ""}`, monitor.current
    ? isActive => loadMonitorThumbnail(() => invoke<ArrayBuffer>("monitor_thumbnail", { monitor: monitor.id }), isActive)
    : null, "image/png");
  return <>{src ? <img key={src} className="loaded-image" src={src} alt=""/> : <div className="thumbnail-placeholder">{failed ? "No preview" : "Loading…"}</div>}{loading && src && <span className="image-loading" role="status" aria-label="Loading preview"/>}</>;
}

/** Monitor arrangement drawn to scale, like Windows display settings. */
function MonitorLayout({ monitors, names, selected, select }: { monitors: Monitor[]; names: MonitorName[]; selected: string | null; select: (id: string) => void }) {
  const left = Math.min(...monitors.map(m => m.x)), top = Math.min(...monitors.map(m => m.y));
  const width = Math.max(...monitors.map(m => m.x + m.width)) - left;
  const height = Math.max(...monitors.map(m => m.y + m.height)) - top;
  return <div className="monitor-layout" style={{ aspectRatio: `${width} / ${height}`, width: `min(100%, 560px, ${Math.round(260 * width / height)}px)` }} role="group" aria-label="Monitor arrangement">
    {monitors.map((monitor, index) => <button key={monitor.id} className={`monitor-tile${selected === monitor.id ? " selected" : ""}`} aria-pressed={selected === monitor.id} title={monitor.name ?? undefined} onClick={() => select(monitor.id)}
      style={{ left: `${(monitor.x - left) / width * 100}%`, top: `${(monitor.y - top) / height * 100}%`, width: `${monitor.width / width * 100}%`, height: `${monitor.height / height * 100}%` }}>
      <MonitorThumb monitor={monitor}/>
      <span className="monitor-tile-label"><strong>{index + 1}</strong>{monitorLabel(monitor.id, monitors, names)}{monitor.primary && <small>Primary</small>}</span>
    </button>)}
  </div>;
}

export function MonitorsPage({ config, busy, saveMonitors }: { config: UtilityConfig; busy: boolean; saveMonitors: (folders: MonitorFolders[], names: MonitorName[]) => Promise<void> }) {
  const { monitors, loading, error, refresh } = useMonitors();
  const { setError, setNotice } = useSnackbar("monitors");
  const [selected, setSelected] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<{ id: string; value: string } | null>(null);
  const [moveTarget, setMoveTarget] = useState<Record<string, string>>({});
  const [counts, setCounts] = useState<Record<string, number | null>>({});
  const [confirm, setConfirm] = useState<Confirmation | null>(null);
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
  const copied = (ok: boolean) => ok ? setNotice("Folder path copied.") : setError("Could not copy the folder path.");
  const copyPath = (path: string) => navigator.clipboard.writeText(path).then(() => copied(true), () => copied(false));
  const openFolder = (path: string) => run(() => invoke("open_folder", { path }));
  const confirmRemove = (id: string, path: string) => setConfirm({ title: "Remove this folder?", message: `${maskFolderPath(path)} will no longer be used by ${monitorLabel(id, monitors, config.monitor_names)}. The images stay on your disk.`, confirm: "Remove", action: () => removeFolder(id, path) });
  const confirmForget = (id: string) => setConfirm({ title: "Forget this monitor?", message: `The folders and name saved for ${monitorLabel(id, monitors, config.monitor_names)} will be removed. The images stay on your disk.`, confirm: "Forget", action: () => forget(id) });
  const folderList = (id: string) => foldersOf(id).map(folder => <div className="monitor-folder" key={folder.path}><FolderOpen size={15}/><PathCopy path={folder.path} copied={copied}/>
    <button className="icon-button" aria-label={`Copy full path of ${maskFolderPath(folder.path)}`} title="Copy path" onClick={() => copyPath(folder.path)}><Copy size={14}/></button>
    <button className="icon-button" aria-label={`Open ${maskFolderPath(folder.path)} in File Explorer`} title="Open folder" onClick={() => openFolder(folder.path)}><ExternalLink size={14}/></button>
    <button className="icon-button danger" aria-label={`Remove ${maskFolderPath(folder.path)}`} title="Remove folder" disabled={busy} onClick={() => confirmRemove(id, folder.path)}><Trash2 size={14}/></button></div>);

  const savedIds = useMemo(() => {
    const ids = [...config.monitor_folders.map(entry => entry.monitor), ...config.monitor_names.map(entry => entry.monitor)];
    return ids.filter((id, index) => ids.findIndex(other => same(other, id)) === index && !monitors.some(monitor => same(monitor.id, id)));
  }, [config.monitor_folders, config.monitor_names, monitors]);
  const moveTargets = monitors.filter(monitor => foldersOf(monitor.id).length === 0);

  return <>{confirm && <ConfirmDialog request={confirm} close={() => setConfirm(null)}/>}<div className="section-title"><div><span className="eyebrow">YOUR SCREENS</span><h2>Connected monitors</h2></div><button className="icon-button" aria-label="Refresh monitors" disabled={loading} onClick={refresh}><RefreshCw size={16}/></button></div>
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
      <div className="monitor-actions">{moveTargets.length > 0 && <div className="monitor-move"><Select label="Move settings to monitor" placeholder="Move to…" value={moveTarget[id] ?? null} disabled={busy} onChange={target => setMoveTarget(targets => ({ ...targets, [id]: target }))} options={moveTargets.map(monitor => ({ value: monitor.id, label: monitorLabel(monitor.id, monitors, config.monitor_names) }))}/><button className="secondary" aria-label="Move settings" disabled={busy || !moveTarget[id]} onClick={() => move(id)}><ArrowRightLeft size={14}/> Move</button></div>}<button className="secondary" disabled={busy} onClick={() => confirmForget(id)}>Forget</button></div></div>)}</div>
      <p className="footnote">A monitor can get a new device path when it is moved to another port. Move its settings to the monitor's new entry.</p></>}
    <p className="footnote">A monitor with its own folders runs its own slideshow; other monitors share the main slideshow. Interval, shuffle, filters, and exclusions are shared by every monitor, and Windows uses one fit mode for the whole desktop. Connected and disconnected monitors are noticed within about 10 seconds. A single image chosen here lasts until the next change.</p>
  </>;
}
