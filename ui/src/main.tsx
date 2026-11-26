import React, { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ArrowLeft, ArrowRight, ChevronRight, FolderOpen, History, Image, LayoutGrid, Layers, Monitor, Paintbrush, Pause, Play, Plus, RefreshCw, Settings, Trash2 } from "lucide-react";
import { TitleBar } from "./TitleBar";
import { Select } from "./Select";
import { applyTheme, type ThemeSetting } from "./theme";
import { Gallery, HistoryPage, type UtilityConfig } from "./Utilities";
import { MonitorStrip, MonitorsPage } from "./Monitors";
import { Editor, HotkeySettings, Profiles } from "./Extended";
import { maskUserName } from "./maskPath";
import { SnackbarProvider, useSnackbar } from "./Snackbar";
import { FitModeSelect, IntervalSelect } from "./SettingControls";
import "./style.css";

type Folder = { path: string; enabled: boolean; recursive: boolean };
type ProfileStatus = { active: string | null; changed: boolean };
type Config = { theme: ThemeSetting; collage: { enabled: boolean; gap: number } } & { version: number; folders: Folder[]; interval_seconds: number; shuffle: boolean; profiles: { name: string }[]; active_profile: string | null; hotkeys: { next: string; previous: string; pause: string } } & UtilityConfig;
type Page = "overview" | "folders" | "gallery" | "monitors" | "history" | "profiles" | "editor" | "settings";
type OverviewStatus = { state: string; current: string | null; position: number | null; total: number | null; combined: string[]; display: string | null };

function App() {
  const [page, setPage] = useState<Page>("overview");
  const [config, setConfig] = useState<Config | null>(null);
  const [status, setStatus] = useState("stopped");
  const { setError, setNotice: setMessage, navigate } = useSnackbar(page);
  const [busy, setBusy] = useState(false);
  const [autostart, setAutostart] = useState(false);
  const [counts, setCounts] = useState<Record<string, number | string>>({});
  // Bumped when the folder watcher changes the collection, so folder counts
  // and the gallery reload without a page change.
  const [collectionVersion, setCollectionVersion] = useState(0);
  const [profileStatus, setProfileStatus] = useState<ProfileStatus>({ active: null, changed: false });
  const [preview, setPreview] = useState("");
  const [imagePath, setImagePath] = useState("");
  // A combined wallpaper: the images shown next to `imagePath` and the file shown.
  const [combined, setCombined] = useState<string[]>([]);
  const [displayPath, setDisplayPath] = useState("");
  const [imagePosition, setImagePosition] = useState<number | null>(null);
  const [imageTotal, setImageTotal] = useState<number | null>(null);
  const statusRequest = useRef(0);

  const refreshStatus = useCallback(async () => {
    const request = ++statusRequest.current;
    try {
      const value = await invoke<OverviewStatus>("overview_status");
      if (request !== statusRequest.current) return;
      setStatus(value.state);
      setImagePath(value.current ?? "");
      setCombined(value.combined ?? []);
      setDisplayPath(value.display ?? "");
      setImagePosition(value.position);
      setImageTotal(value.total);
    } catch { if (request === statusRequest.current) { setStatus("stopped"); setImagePath(""); setCombined([]); setDisplayPath(""); setImagePosition(null); setImageTotal(null); } }
  }, []);

  useEffect(() => {
    invoke<Config>("get_config").then(value => { setConfig(value); return invoke<string | null>("config_warning"); }).then(warning => { if (warning) setError(warning); }).catch(e => setError(String(e)));
    invoke<boolean>("get_autostart").then(setAutostart).catch(() => {});
  }, [refreshStatus]);

  // The app's own slideshow sends "slideshow-changed" events, so polling is
  // only a slow fallback for a slideshow run from an older CLI that cannot
  // hand over. Closing the window destroys this page, and nothing is polled
  // or refreshed while it is minimized.
  useEffect(() => {
    let timer: number | undefined; let active = true; const disposers: Array<() => void> = [];
    const visible = () => !document.hidden;
    const update = () => {
      if (visible() && timer === undefined) { refreshStatus(); timer = window.setInterval(refreshStatus, 30000); }
      else if (!visible() && timer !== undefined) { window.clearInterval(timer); timer = undefined; }
    };
    const keep = (dispose: () => void) => { if (active) disposers.push(dispose); else dispose(); };
    document.addEventListener("visibilitychange", update);
    listen("slideshow-changed", () => { if (visible()) refreshStatus(); }).then(keep).catch(() => {});
    listen("collection-changed", () => setCollectionVersion(version => version + 1)).then(keep).catch(() => {});
    update();
    return () => { active = false; disposers.forEach(dispose => dispose()); document.removeEventListener("visibilitychange", update); if (timer !== undefined) window.clearInterval(timer); };
  }, [refreshStatus]);

  useEffect(() => {
    let unlisten: (() => void) | undefined; let active = true;
    listen("config-changed", () => { invoke<Config>("get_config").then(setConfig).catch(e => setError(String(e))); refreshStatus(); }).then(dispose => { if (active) unlisten = dispose; else dispose(); }).catch(() => {});
    return () => { active = false; unlisten?.(); };
  }, [refreshStatus, setError]);

  useEffect(() => {
    const shown = displayPath || imagePath;
    if (!shown) { setPreview(""); return; }
    let active = true; let url = "";
    setPreview("");
    invoke<ArrayBuffer>("current_preview", { path: shown }).then(bytes => {
      if (!active) return;
      url = URL.createObjectURL(new Blob([new Uint8Array(bytes)])); setPreview(url);
    }).catch(() => { if (active) setPreview(""); });
    return () => { active = false; if (url) URL.revokeObjectURL(url); };
  }, [imagePath, displayPath]);

  const scanFolders = useCallback(async (folders: Folder[]) => {
    const entries = await Promise.all(folders.map(async f => {
      try { return [f.path, await invoke<number>("folder_count", { path: f.path, recursive: f.recursive })] as const; }
      catch { return [f.path, "Unavailable"] as const; }
    }));
    setCounts(Object.fromEntries(entries));
  }, []);
  useEffect(() => { if (config) scanFolders(config.folders); }, [config?.folders, collectionVersion, scanFolders]);

  async function save(next: Config, propagateError = false) {
    setBusy(true); setError(""); setMessage("");
    try { const result = await invoke<string>("save_config", { config: next }); setConfig(next); setMessage(result); await refreshStatus(); }
    catch (e) { try { setConfig(await invoke<Config>("get_config")); } catch { /* Preserve the original failure. */ } setError(String(e)); if (propagateError) throw e; }
    finally { setBusy(false); }
  }
  async function act(action: string) {
    setBusy(true); setError(""); setMessage("");
    try {
      if (action === "start") { await invoke("start_service"); await new Promise(resolve => setTimeout(resolve, 700)); const failure = await invoke<string | null>("service_error"); if (failure) throw failure; }
      else await invoke("control", { action });
      await refreshStatus();
      if (action === "reload") setMessage("Collection rescanned. New images join the current cycle.");
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }
  async function addFolder() {
    if (!config) return;
    try { const path = await invoke<string | null>("pick_folder"); if (path && !config.folders.some(f => f.path.toLowerCase() === path.toLowerCase())) await save({ ...config, folders: [...config.folders, { path, enabled: true, recursive: true }] }); }
    catch (e) { setError(String(e)); }
  }
  // The active profile is shown on every page, so reload it after each config change.
  useEffect(() => { if (config) invoke<ProfileStatus>("profile_status").then(setProfileStatus).catch(() => {}); }, [config]);
  useEffect(() => applyTheme(config?.theme ?? "system"), [config?.theme]);
  const ownMonitors = !!config?.monitor_folders.some(entry => entry.folders.some(folder => folder.enabled));
  function navigatePage(next: Page) { navigate(next); setPage(next); }
  const fileName = imagePath.split(/[\\/]/).pop();
  const running = status === "running";
  const filtersActive = config && (config.excluded.length > 0 || config.filter.min_width > 0 || config.filter.min_height > 0 || config.filter.orientation !== "any" || config.filter.aspect_ratio !== "any");
  const nav: { id: Page; label: string; icon: typeof LayoutGrid }[] = [
    { id: "overview", label: "Overview", icon: LayoutGrid }, { id: "folders", label: "Folders", icon: FolderOpen }, { id: "gallery", label: "Gallery", icon: Image }, { id: "monitors", label: "Monitors", icon: Monitor }, { id: "history", label: "History", icon: History }, { id: "profiles", label: "Profiles", icon: Layers }, { id: "editor", label: "Editor", icon: Paintbrush }, { id: "settings", label: "Settings", icon: Settings },
  ];
  return <div className="app-frame"><TitleBar/><div className="app-shell">
    <aside className="sidebar">
      <div className="nav-caption">WORKSPACE</div><nav aria-label="Main navigation">{nav.map(({ id, label, icon: Icon }) => <button key={id} className={page === id ? "nav active" : "nav"} onClick={() => navigatePage(id)} aria-current={page === id ? "page" : undefined}><Icon size={17}/>{label}{page === id && <ChevronRight size={15} className="nav-arrow"/>}</button>)}</nav>
      <div className="sidebar-bottom"><span className="local-dot"/> LOCAL MODE <small>Made for your space.</small></div>
    </aside>
    <main><header className="topline"><span>YOUR PERSONAL WALLPAPER MANAGER</span>{config && config.profiles.length > 0 && <button className={`profile-pill${profileStatus.changed ? " changed" : ""}`} onClick={() => navigatePage("profiles")} title={profileStatus.changed ? "Settings differ from the saved profile. Open Profiles to update it." : "Open Profiles"}><Layers size={13}/>{profileStatus.active ? <>Profile: <strong>{profileStatus.active}</strong>{profileStatus.changed && <em>· changed</em>}</> : "No profile"}</button>}<span className="topline-right"><span className="local-dot"/> Offline by design</span></header>
      <div className="content"><div className="heading"><div><div className="eyebrow">{page === "overview" ? "YOUR SPACE" : page === "settings" ? "PREFERENCES" : "YOUR COLLECTION"}</div><h1>{page === "overview" ? "A little change of scenery." : page === "folders" ? "Your folders." : page === "gallery" ? "A collection of moments." : page === "history" ? "Take another look." : page === "monitors" ? "Your screens." : "Make it yours."}</h1><p>{page === "overview" ? "A fresh perspective, right on your desktop." : page === "folders" ? "Choose the places where your wallpapers live." : page === "gallery" ? "Find a favorite for your desktop." : page === "history" ? "Revisit wallpapers you have enjoyed." : page === "monitors" ? "Give each display its own view." : "Set the rhythm that feels right for you."}</p></div><span className={`status ${status}`}><span/> {status === "running" ? "Playing" : status === "paused" ? "Paused" : "Stopped"}</span></div>
        {!config ? <div className="panel empty">Loading your workspace…</div> : <>
        {page === "overview" && <><div className="section-title"><div><span className="eyebrow">CURRENT WALLPAPER</span><h2>Your desktop, right now</h2></div><span className="subtle">A moment for yourself</span></div>
          <div className="preview" style={preview ? { backgroundImage: `linear-gradient(0deg, rgba(12,13,25,.85), transparent 58%), url("${preview}")` } : undefined}><div className="preview-content"><div className="preview-tag"><span className="local-dot"/> {!imagePath ? "YOUR NEXT FAVORITE" : ownMonitors ? "MAIN SLIDESHOW" : "NOW SHOWING"}</div><h3 className={fileName ? "selectable" : undefined}>{fileName || "A blank canvas."}{combined.length > 0 && <span className="combined-badge" title={combined.join("\n")}>+{combined.length} {combined.length === 1 ? "image" : "images"}</span>}</h3><p className={imagePath ? "selectable" : undefined}>{imagePath ? imagePath : "Add a folder to bring your collection to life."}</p></div><span className="preview-number" aria-label="Image position in slideshow">{imagePosition !== null && imageTotal !== null ? `${imagePosition} / ${imageTotal}` : "— / —"}</span></div>
          <MonitorStrip config={config}/>
          <div className="footnote overview-collection">{filtersActive && <>Image filters or exclusions are active. <button className="text-link" onClick={() => navigatePage("settings")}>Review settings <ArrowRight size={14}/></button></>}{status !== "stopped" && <button className="text-link" disabled={busy} onClick={() => act("reload")} title="Rescans enabled folders and keeps the current cycle"><RefreshCw size={14}/> Rescan collection</button>}</div>
          <div className="player"><div><strong>Slideshow</strong><small>{status === "stopped" ? "Ready when you are" : `Changing every ${config.interval_seconds / 60} minutes`}</small></div><div className="player-actions"><button className="icon-button" disabled={busy || status === "stopped"} onClick={() => act("previous")} aria-label="Previous wallpaper"><ArrowLeft size={17}/></button>{status === "stopped" ? <button className="primary" disabled={busy || !config.folders.some(f => f.enabled)} onClick={() => act("start")}><Play size={16}/> Start slideshow</button> : <button className="primary" disabled={busy} onClick={() => act(running ? "pause" : "resume")}>{running ? <Pause size={16}/> : <Play size={16}/>} {running ? "Pause" : "Resume"}</button>}<button className="icon-button" disabled={busy || status === "stopped"} onClick={() => act("next")} aria-label="Next wallpaper"><ArrowRight size={17}/></button></div></div>
          <section className="quick-settings" aria-labelledby="quick-settings-title"><div className="section-title"><div><span className="eyebrow">SLIDESHOW</span><h2 id="quick-settings-title">Quick settings</h2></div></div><div className="panel settings-panel"><div className="setting-row"><div><strong>Change picture every</strong><small>Set the time between wallpapers.</small></div><IntervalSelect value={config.interval_seconds} disabled={busy} onChange={interval_seconds => save({ ...config, interval_seconds })}/></div><div className="setting-row"><div><strong>Fit mode</strong><small>Choose how wallpapers fit your screen.</small></div><FitModeSelect value={config.fit_mode} disabled={busy} onChange={fit_mode => save({ ...config, fit_mode })}/></div></div></section>
          <div className="overview-grid"><div className="summary-card"><div className="summary-icon"><FolderOpen size={18}/></div><span>YOUR COLLECTION</span><strong>{config.folders.filter(f => f.enabled).length} <small>active folders</small></strong><button className="text-link" onClick={() => navigatePage("folders")}>Manage folders <ArrowRight size={14}/></button></div><div className="summary-card"><div className="summary-icon"><RefreshCw size={18}/></div><span>THE FLOW</span><strong>{config.shuffle ? "Shuffle" : "In order"} <small>every {config.interval_seconds / 60} min</small></strong><button className="text-link" onClick={() => navigatePage("settings")}>Adjust settings <ArrowRight size={14}/></button></div></div></>}
        {page === "folders" && <><div className="section-title"><div><span className="eyebrow">COLLECTION SOURCES</span><h2>Folders <span className="count-pill">{config.folders.length}</span></h2></div><button className="primary" disabled={busy} onClick={addFolder}><Plus size={16}/> Add folder</button></div>
          {config.folders.length === 0 ? <div className="panel empty"><div className="empty-icon"><FolderOpen size={25}/></div><h3>It starts with a folder.</h3><p>Add a folder of images and make this space your own.</p><button className="primary" disabled={busy} onClick={addFolder}><Plus size={16}/> Add your first folder</button></div> : <div className="panel folder-list">{config.folders.map((folder, index) => <div className="folder-row" key={folder.path}><div className="folder-icon"><FolderOpen size={19}/></div><div className="folder-info"><strong className="selectable" title={folder.path}>{folder.path.split(/[\\/]/).pop() || folder.path}</strong><small className="selectable" title={folder.path}>{folder.path}</small><span>{counts[folder.path] === undefined ? "Scanning…" : typeof counts[folder.path] === "number" ? `${counts[folder.path]} images` : counts[folder.path]}</span></div><label className="inline-check"><input type="checkbox" checked={folder.recursive} disabled={busy} onChange={e => save({ ...config, folders: config.folders.map((f, i) => i === index ? { ...f, recursive: e.target.checked } : f) })}/> Include subfolders</label><label className="switch" title="Enable folder"><input type="checkbox" aria-label={`Enable ${folder.path}`} checked={folder.enabled} disabled={busy} onChange={e => save({ ...config, folders: config.folders.map((f, i) => i === index ? { ...f, enabled: e.target.checked } : f) })}/><span/></label><button className="icon-button danger" aria-label={`Remove ${folder.path}`} disabled={busy} onClick={() => save({ ...config, folders: config.folders.filter((_, i) => i !== index) })}><Trash2 size={16}/></button></div>)}</div>}
          <div className="footnote"><Image size={16}/> JPG, JPEG, PNG and BMP images are supported.</div></>}
        {page === "gallery" && <Gallery config={config} busy={busy} collectionVersion={collectionVersion} openSettings={() => navigatePage("settings")} exclude={async path => { await save({ ...config, excluded: [...config.excluded, path] }, true); }} restore={async path => { await save({ ...config, excluded: config.excluded.filter(item => item !== path) }, true); }}/>}
        {page === "history" && <HistoryPage names={config.monitor_names}/>}
        {page === "monitors" && <MonitorsPage config={config} busy={busy} saveMonitors={(monitor_folders, monitor_names) => save({ ...config, monitor_folders, monitor_names })}/>}
        {page === "profiles" && <Profiles config={config} status={profileStatus} refresh={async () => { setConfig(await invoke<Config>("get_config")); await refreshStatus(); }}/>}
        <div hidden={page !== "editor"}><Editor active={page === "editor"}/></div>
        {page === "settings" && <><div className="section-title"><div><span className="eyebrow">SLIDESHOW</span><h2>Find your rhythm</h2></div></div><div className="panel settings-panel"><div className="setting-row"><div><strong>Change picture every</strong><small>How long each wallpaper stays on your desktop.</small></div><IntervalSelect value={config.interval_seconds} disabled={busy} onChange={interval_seconds => save({ ...config, interval_seconds })}/></div><div className="setting-row"><div><strong>Shuffle wallpapers</strong><small>Enjoy your collection in a different order each time. Images do not repeat within a cycle.</small></div><label className="switch"><input type="checkbox" aria-label="Shuffle wallpapers" checked={config.shuffle} disabled={busy} onChange={e => save({ ...config, shuffle: e.target.checked })}/><span/></label></div><div className="setting-row"><div><strong>Start with Windows</strong><small>Open MindsWall when you sign in to this account.</small></div><label className="switch"><input type="checkbox" aria-label="Start with Windows" checked={autostart} disabled={busy} onChange={async e => { const enabled = e.target.checked; setBusy(true); try { await invoke("set_autostart", { enabled }); setAutostart(enabled); setMessage("Startup preference saved."); } catch (err) { setError(String(err)); } finally { setBusy(false); } }}/><span/></label></div></div><div className="footnote"><span className="local-dot"/> Your settings and images stay on this device.</div></>}
        {page === "settings" && <UtilitySettings config={config} busy={busy} save={save}/>}
        {page === "settings" && <HotkeySettings hotkeys={config.hotkeys} refresh={async () => { setConfig(await invoke<Config>("get_config")); }}/>}
        </>}
      </div><footer>MINDSWALL <span>•</span> A LITTLE SPACE OF YOUR OWN <span className="footer-version">VERSION 0.1.0</span></footer>
    </main>
  </div></div>;
}

function UtilitySettings({ config, busy, save }: { config: Config; busy: boolean; save: (config: Config) => Promise<void> }) {
  const [width, setWidth] = useState(String(config.filter.min_width));
  const [height, setHeight] = useState(String(config.filter.min_height));
  const [logPath, setLogPath] = useState<string | null>(null);
  const { setError, setNotice } = useSnackbar("settings");
  useEffect(() => { invoke<string>("log_location").then(setLogPath).catch(() => setLogPath(null)); }, []);
  async function copyLogPath() {
    if (!logPath) return;
    try { await navigator.clipboard.writeText(logPath); setNotice("Log path copied."); } catch { setError("Could not copy the log path."); }
  }
  async function applyFilter() {
    const w = Number(width), h = Number(height);
    if (!Number.isInteger(w) || !Number.isInteger(h) || w < 0 || h < 0 || w > 100000 || h > 100000) { setError("Enter whole numbers between 0 and 100000."); return; }
    await save({ ...config, filter: { ...config.filter, min_width: w, min_height: h } });
  }
  return <><div className="section-title"><div><span className="eyebrow">APPEARANCE</span><h2>Look and feel</h2></div></div><div className="panel settings-panel">
    <div className="setting-row"><div><strong>Theme</strong><small>System follows the Windows app mode.</small></div><Select label="Theme" value={config.theme} disabled={busy} onChange={theme => save({ ...config, theme })} options={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]}/></div>
  </div>
  <div className="section-title utility-heading"><div><span className="eyebrow">WALLPAPER UTILITIES</span><h2>Image preferences</h2></div></div><div className="panel settings-panel">
    <div className="setting-row"><div><strong>Fit mode</strong><small>How each wallpaper fills your screen.</small></div><FitModeSelect value={config.fit_mode} disabled={busy} onChange={fit_mode => save({ ...config, fit_mode })}/></div>
    <div className="setting-row"><div><strong>Orientation</strong><small>Only show images with this orientation.</small></div><Select label="Image orientation" value={config.filter.orientation} disabled={busy} onChange={orientation => save({ ...config, filter: { ...config.filter, orientation } })} options={[{ value: "any", label: "Any" }, { value: "landscape", label: "Landscape" }, { value: "portrait", label: "Portrait" }, { value: "square", label: "Square" }]}/></div>
    <div className="setting-row"><div><strong>Aspect ratio</strong><small>Allow a 5% difference from the selected ratio.</small></div><Select label="Image aspect ratio" value={config.filter.aspect_ratio} disabled={busy} onChange={aspect_ratio => save({ ...config, filter: { ...config.filter, aspect_ratio } })} options={[{ value: "any", label: "Any" }, { value: "wide", label: "16:9" }, { value: "wide_ten", label: "16:10" }, { value: "standard", label: "4:3" }, { value: "square", label: "1:1" }]}/></div>
    <div className="setting-row"><div><strong>Minimum resolution</strong><small>Use 0 to accept any width or height.</small></div><div className="resolution"><input aria-label="Minimum width" type="number" min="0" max="100000" value={width} onChange={e => setWidth(e.target.value)}/><span>×</span><input aria-label="Minimum height" type="number" min="0" max="100000" value={height} onChange={e => setHeight(e.target.value)}/><button className="secondary" disabled={busy} onClick={applyFilter}>Apply</button></div></div>
    <div className="setting-row"><div><strong>Log file</strong><small className="log-path">{logPath ? maskUserName(logPath) : "Log location unavailable."}</small>{logPath && <small>The user name is hidden; <b>Copy path</b> copies the full path.</small>}</div><button className="secondary" disabled={!logPath} onClick={copyLogPath}>Copy path</button></div>
    <div className="setting-row"><div><strong>Combine portrait images</strong><small>On a landscape monitor, show portrait images side by side over a blurred background. Works with every fit mode.</small></div><label className="switch" title="Combine portrait images"><input type="checkbox" aria-label="Combine portrait images" checked={config.collage.enabled} disabled={busy} onChange={e => save({ ...config, collage: { ...config.collage, enabled: e.target.checked } })}/><span/></label></div>
    <div className="setting-row"><div><strong>Space between images</strong><small>Used when portrait images are combined.</small></div><Select label="Space between combined images" value={config.collage.gap} disabled={busy || !config.collage.enabled} onChange={gap => save({ ...config, collage: { ...config.collage, gap } })} options={[0, 2, 4, 8, 12, 16, 24, 32].map(gap => ({ value: gap, label: gap === 0 ? "None" : `${gap} px` }))}/></div>
    <div className="setting-row"><div><strong>Excluded images</strong><small>{config.excluded.length} images excluded from your collection.</small></div><button className="secondary" disabled={busy || config.excluded.length === 0} onClick={() => save({ ...config, excluded: [] })}>Restore all</button></div>
  </div></>;
}

createRoot(document.getElementById("root")!).render(<React.StrictMode><SnackbarProvider><App/></SnackbarProvider></React.StrictMode>);
