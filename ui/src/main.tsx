import React, { useCallback, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowLeft, ArrowRight, Check, ChevronRight, FolderOpen, Image, LayoutGrid, Pause, Play, Plus, RefreshCw, Settings, Sparkles, Trash2 } from "lucide-react";
import "./style.css";

type Folder = { path: string; enabled: boolean; recursive: boolean };
type Config = { version: number; folders: Folder[]; interval_seconds: number; shuffle: boolean };
type Page = "overview" | "folders" | "settings";

function App() {
  const [page, setPage] = useState<Page>("overview");
  const [config, setConfig] = useState<Config | null>(null);
  const [status, setStatus] = useState("stopped");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [autostart, setAutostart] = useState(false);
  const [counts, setCounts] = useState<Record<string, number | string>>({});
  const [preview, setPreview] = useState("");
  const [imagePath, setImagePath] = useState("");

  const refreshStatus = useCallback(async () => {
    try {
      const value = await invoke<string>("control", { action: "status" });
      const split = value.indexOf(": ");
      setStatus(value.slice(0, split));
      setImagePath(value.slice(split + 2));
    } catch { setStatus("stopped"); setImagePath(""); }
  }, []);

  useEffect(() => {
    invoke<Config>("get_config").then(setConfig).catch(e => setError(String(e)));
    invoke<boolean>("get_autostart").then(setAutostart).catch(() => {});
    refreshStatus();
    const timer = window.setInterval(refreshStatus, 3000);
    return () => window.clearInterval(timer);
  }, [refreshStatus]);

  useEffect(() => {
    if (!imagePath) { setPreview(""); return; }
    let active = true; let url = "";
    invoke<number[]>("current_preview").then(bytes => {
      if (!active) return;
      url = URL.createObjectURL(new Blob([new Uint8Array(bytes)])); setPreview(url);
    }).catch(() => { if (active) setPreview(""); });
    return () => { active = false; if (url) URL.revokeObjectURL(url); };
  }, [imagePath]);

  const scanFolders = useCallback(async (folders: Folder[]) => {
    const entries = await Promise.all(folders.map(async f => {
      try { return [f.path, await invoke<number>("folder_count", { path: f.path, recursive: f.recursive })] as const; }
      catch { return [f.path, "Unavailable"] as const; }
    }));
    setCounts(Object.fromEntries(entries));
  }, []);
  useEffect(() => { if (config) scanFolders(config.folders); }, [config?.folders, scanFolders]);

  async function save(next: Config) {
    setBusy(true); setError(""); setMessage("");
    try { const result = await invoke<string>("save_config", { config: next }); setConfig(next); setMessage(result); await refreshStatus(); }
    catch (e) { setConfig(await invoke<Config>("get_config")); setError(String(e)); }
    finally { setBusy(false); }
  }
  async function act(action: string) {
    setBusy(true); setError(""); setMessage("");
    try {
      if (action === "start") { await invoke("start_service"); await new Promise(resolve => setTimeout(resolve, 700)); const failure = await invoke<string | null>("service_error"); if (failure) throw failure; }
      else await invoke("control", { action });
      await refreshStatus();
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }
  async function addFolder() {
    if (!config) return;
    try { const path = await open({ directory: true, multiple: false }); if (path && !config.folders.some(f => f.path.toLowerCase() === path.toLowerCase())) await save({ ...config, folders: [...config.folders, { path, enabled: true, recursive: true }] }); }
    catch (e) { setError(String(e)); }
  }
  const fileName = imagePath.split(/[\\/]/).pop();
  const running = status === "running";
  const nav: { id: Page; label: string; icon: typeof LayoutGrid }[] = [
    { id: "overview", label: "Overview", icon: LayoutGrid }, { id: "folders", label: "Folders", icon: FolderOpen }, { id: "settings", label: "Settings", icon: Settings },
  ];
  return <div className="app-shell">
    <aside className="sidebar"><div className="brand"><span className="brand-icon"><Sparkles size={17}/></span><div><strong>MindsWall</strong><small>YOUR DESKTOP, CURATED</small></div></div>
      <div className="nav-caption">WORKSPACE</div><nav aria-label="Main navigation">{nav.map(({ id, label, icon: Icon }) => <button key={id} className={page === id ? "nav active" : "nav"} onClick={() => setPage(id)} aria-current={page === id ? "page" : undefined}><Icon size={17}/>{label}{page === id && <ChevronRight size={15} className="nav-arrow"/>}</button>)}</nav>
      <div className="sidebar-bottom"><span className="local-dot"/> LOCAL MODE <small>Made for your space.</small></div>
    </aside>
    <main><header className="topline"><span>YOUR PERSONAL WALLPAPER MANAGER</span><span className="topline-right"><span className="local-dot"/> Offline by design</span></header>
      <div className="content"><div className="heading"><div><div className="eyebrow">{page === "overview" ? "YOUR SPACE" : page === "folders" ? "YOUR COLLECTION" : "PREFERENCES"}</div><h1>{page === "overview" ? "A little change of scenery." : page === "folders" ? "Your folders." : "Make it yours."}</h1><p>{page === "overview" ? "A fresh perspective, right on your desktop." : page === "folders" ? "Choose the places where your wallpapers live." : "Set the rhythm that feels right for you."}</p></div><span className={`status ${status}`}><span/> {status === "running" ? "Playing" : status === "paused" ? "Paused" : "Stopped"}</span></div>
        {error && <div className="notice error" role="alert">{error}<button onClick={() => setError("")} aria-label="Dismiss error">×</button></div>}
        {message && <div className="notice success" role="status"><Check size={16}/> {message}<button onClick={() => setMessage("")} aria-label="Dismiss message">×</button></div>}
        {!config ? <div className="panel empty">Loading your workspace…</div> : <>
        {page === "overview" && <><div className="section-title"><div><span className="eyebrow">CURRENT WALLPAPER</span><h2>Your desktop, right now</h2></div><span className="subtle">A moment for yourself</span></div>
          <div className="preview" style={preview ? { backgroundImage: `linear-gradient(0deg, rgba(12,13,25,.85), transparent 58%), url("${preview}")` } : undefined}><div className="preview-content"><div className="preview-tag"><span className="local-dot"/> {imagePath ? "NOW SHOWING" : "YOUR NEXT FAVORITE"}</div><h3>{fileName || "A blank canvas."}</h3><p>{imagePath ? imagePath : "Add a folder to bring your collection to life."}</p></div><span className="preview-number">01 / 01</span></div>
          <div className="player"><div><strong>Slideshow</strong><small>{status === "stopped" ? "Ready when you are" : `Changing every ${config.interval_seconds / 60} minutes`}</small></div><div className="player-actions"><button className="icon-button" disabled={busy || status === "stopped"} onClick={() => act("previous")} aria-label="Previous wallpaper"><ArrowLeft size={17}/></button>{status === "stopped" ? <button className="primary" disabled={busy || !config.folders.some(f => f.enabled)} onClick={() => act("start")}><Play size={16}/> Start slideshow</button> : <button className="primary" disabled={busy} onClick={() => act(running ? "pause" : "resume")}>{running ? <Pause size={16}/> : <Play size={16}/>} {running ? "Pause" : "Resume"}</button>}<button className="icon-button" disabled={busy || status === "stopped"} onClick={() => act("next")} aria-label="Next wallpaper"><ArrowRight size={17}/></button></div></div>
          <div className="overview-grid"><div className="summary-card"><div className="summary-icon"><FolderOpen size={18}/></div><span>YOUR COLLECTION</span><strong>{config.folders.filter(f => f.enabled).length} <small>active folders</small></strong><button className="text-link" onClick={() => setPage("folders")}>Manage folders <ArrowRight size={14}/></button></div><div className="summary-card"><div className="summary-icon"><RefreshCw size={18}/></div><span>THE FLOW</span><strong>{config.shuffle ? "Shuffle" : "In order"} <small>every {config.interval_seconds / 60} min</small></strong><button className="text-link" onClick={() => setPage("settings")}>Adjust settings <ArrowRight size={14}/></button></div></div></>}
        {page === "folders" && <><div className="section-title"><div><span className="eyebrow">COLLECTION SOURCES</span><h2>Folders <span className="count-pill">{config.folders.length}</span></h2></div><button className="primary" disabled={busy} onClick={addFolder}><Plus size={16}/> Add folder</button></div>
          {config.folders.length === 0 ? <div className="panel empty"><div className="empty-icon"><FolderOpen size={25}/></div><h3>It starts with a folder.</h3><p>Add a folder of images and make this space your own.</p><button className="primary" disabled={busy} onClick={addFolder}><Plus size={16}/> Add your first folder</button></div> : <div className="panel folder-list">{config.folders.map((folder, index) => <div className="folder-row" key={folder.path}><div className="folder-icon"><FolderOpen size={19}/></div><div className="folder-info"><strong title={folder.path}>{folder.path.split(/[\\/]/).pop() || folder.path}</strong><small title={folder.path}>{folder.path}</small><span>{counts[folder.path] === undefined ? "Scanning…" : typeof counts[folder.path] === "number" ? `${counts[folder.path]} images` : counts[folder.path]}</span></div><label className="inline-check"><input type="checkbox" checked={folder.recursive} disabled={busy} onChange={e => save({ ...config, folders: config.folders.map((f, i) => i === index ? { ...f, recursive: e.target.checked } : f) })}/> Include subfolders</label><label className="switch" title="Enable folder"><input type="checkbox" aria-label={`Enable ${folder.path}`} checked={folder.enabled} disabled={busy} onChange={e => save({ ...config, folders: config.folders.map((f, i) => i === index ? { ...f, enabled: e.target.checked } : f) })}/><span/></label><button className="icon-button danger" aria-label={`Remove ${folder.path}`} disabled={busy} onClick={() => save({ ...config, folders: config.folders.filter((_, i) => i !== index) })}><Trash2 size={16}/></button></div>)}</div>}
          <div className="footnote"><Image size={16}/> JPG, JPEG, PNG and BMP images are supported.</div></>}
        {page === "settings" && <><div className="section-title"><div><span className="eyebrow">SLIDESHOW</span><h2>Find your rhythm</h2></div></div><div className="panel settings-panel"><div className="setting-row"><div><strong>Change picture every</strong><small>How long each wallpaper stays on your desktop.</small></div><select aria-label="Change picture every" value={config.interval_seconds} disabled={busy} onChange={e => save({ ...config, interval_seconds: Number(e.target.value) })}>{[60,300,600,900,1800,3600,21600,43200,86400].map(n => <option key={n} value={n}>{n >= 3600 ? `${n / 3600} ${n === 3600 ? "hour" : "hours"}` : `${n / 60} ${n === 60 ? "minute" : "minutes"}`}</option>)}</select></div><div className="setting-row"><div><strong>Shuffle wallpapers</strong><small>Enjoy your collection in a different order each time. Images do not repeat within a cycle.</small></div><label className="switch"><input type="checkbox" aria-label="Shuffle wallpapers" checked={config.shuffle} disabled={busy} onChange={e => save({ ...config, shuffle: e.target.checked })}/><span/></label></div><div className="setting-row"><div><strong>Start with Windows</strong><small>Open MindsWall when you sign in to this account.</small></div><label className="switch"><input type="checkbox" aria-label="Start with Windows" checked={autostart} disabled={busy} onChange={async e => { const enabled = e.target.checked; setBusy(true); try { await invoke("set_autostart", { enabled }); setAutostart(enabled); setMessage("Startup preference saved."); } catch (err) { setError(String(err)); } finally { setBusy(false); } }}/><span/></label></div></div><div className="footnote"><span className="local-dot"/> Your settings and images stay on this device.</div></>}
        </>}
      </div><footer>MINDSWALL <span>•</span> A LITTLE SPACE OF YOUR OWN <span className="footer-version">VERSION 0.1.0</span></footer>
    </main>
  </div>;
}

createRoot(document.getElementById("root")!).render(<React.StrictMode><App/></React.StrictMode>);
