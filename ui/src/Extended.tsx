import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, Download, Upload } from "lucide-react";
import { useSnackbar } from "./Snackbar";

type Profile = { name: string };
type ExtendedConfig = { profiles: Profile[]; active_profile: string | null };
type Hotkeys = { next: string; previous: string; pause: string };
type EditResult = { path: string; preview: number[] };
type Props = { config: ExtendedConfig; refresh: () => Promise<void> };

export function Profiles({ config, refresh }: Props) {
  const [name, setName] = useState(""); const { setError, setNotice } = useSnackbar("profiles"); const [busy, setBusy] = useState(false); const [removing, setRemoving] = useState("");
  async function action(command: string, args: Record<string, unknown>, message: string) {
    setBusy(true); setError(""); setNotice("");
    try { await invoke(command, args); await refresh(); setNotice(message); return true; } catch (e) { await refresh().catch(() => {}); setError(String(e)); return false; } finally { setBusy(false); }
  }
  async function exportFile() { try { const path = await saveDialog({ defaultPath: "mindswall-backup.json", filters: [{ name: "JSON", extensions: ["json"] }] }); if (path) await action("export_backup", { path }, "Backup exported."); } catch (e) { setError(String(e)); } }
  async function importFile() { try { const path = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] }); if (path) await action("import_backup", { path }, "Backup imported."); } catch (e) { setError(String(e)); } }
  return <><div className="section-title"><div><span className="eyebrow">YOUR COLLECTIONS</span><h2>Wallpaper profiles</h2></div></div>
    <div className="panel settings-panel"><div className="setting-row"><div><strong>Save current settings</strong><small>Keep your folders, slideshow and image preferences together.</small></div><div className="profile-create"><input aria-label="Profile name" maxLength={80} value={name} onChange={e => setName(e.target.value)} placeholder="Profile name"/><button className="primary" disabled={busy || !name.trim()} onClick={async () => { if (await action("save_profile", { name }, "Profile saved.")) setName(""); }}>Save</button></div></div>
    {config.profiles.length === 0 ? <p className="extended-empty">No profiles yet. Save your current setup to get started.</p> : config.profiles.map(profile => <div className="setting-row" key={profile.name}><div><strong>{profile.name}{config.active_profile === profile.name ? " · Active" : ""}</strong><small>Saved wallpaper collection</small></div><div className="profile-create"><button className="secondary" disabled={busy} onClick={() => action("apply_profile", { name: profile.name }, "Profile applied.")}>Use profile</button>{removing === profile.name ? <><button className="secondary" disabled={busy} onClick={async () => { await action("delete_profile", { name: profile.name }, "Profile removed."); setRemoving(""); }}>Confirm remove</button><button className="secondary" onClick={() => setRemoving("")}>Cancel</button></> : <button className="secondary" disabled={busy} onClick={() => setRemoving(profile.name)}>Remove</button>}</div></div>)}</div>
    <div className="section-title utility-heading"><div><span className="eyebrow">PORTABILITY</span><h2>Backup & restore</h2></div></div><div className="panel setting-row"><div><strong>Your settings, safely stored</strong><small>Export folders, preferences and profiles as JSON. Import replaces current settings and profiles.</small></div><div className="profile-create"><button className="secondary" disabled={busy} onClick={exportFile}><Download size={14}/> Export</button><button className="secondary" disabled={busy} onClick={importFile}><Upload size={14}/> Import</button></div></div>
  </>;
}

export function Editor() {
  const [source, setSource] = useState(""); const [edited, setEdited] = useState(""); const [preview, setPreview] = useState("");
  const [width, setWidth] = useState(""); const [height, setHeight] = useState(""); const [crop, setCrop] = useState(false);
  const [x, setX] = useState("0"); const [y, setY] = useState("0"); const [cropWidth, setCropWidth] = useState(""); const [cropHeight, setCropHeight] = useState("");
  const [blur, setBlur] = useState(0); const [dim, setDim] = useState(0); const { setError, setNotice } = useSnackbar("editor"); const [busy, setBusy] = useState(false);
  const mounted = useRef(false);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => () => { if (preview) URL.revokeObjectURL(preview); }, [preview]);
  async function choose() { try { const path = await open({ multiple: false, filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "bmp"] }] }); if (path) { setSource(path); setEdited(""); setPreview(""); setError(""); } } catch (e) { setError(String(e)); } }
  async function generate() {
    setError(""); setNotice("");
    const number = (v: string) => /^\d+$/.test(v) ? Number(v) : null;
    const w = width ? number(width) : null, h = height ? number(height) : null;
    const cropValues = [x, y, cropWidth, cropHeight].map(number);
    if ((!!width !== !!height) || (width && (w === null || h === null)) || (crop && cropValues.some(v => v === null))) { setError("Enter valid whole numbers for both resize dimensions and all crop fields."); return; }
    setEdited(""); setBusy(true);
    try {
      const result = await invoke<EditResult>("edit_image", { path: source, options: { width: w, height: h, crop: crop ? { x: cropValues[0], y: cropValues[1], width: cropValues[2], height: cropValues[3] } : null, blur, dim } });
      if (!mounted.current) return;
      setEdited(result.path); setPreview(URL.createObjectURL(new Blob([new Uint8Array(result.preview)], { type: "image/png" }))); setNotice("Preview created. Your original image is unchanged.");
    } catch (e) { if (mounted.current) setError(String(e)); } finally { if (mounted.current) setBusy(false); }
  }
  return <><div className="section-title"><div><span className="eyebrow">MAKE IT YOURS</span><h2>Image editor</h2></div><button className="primary" disabled={busy} onClick={choose}><FolderOpen size={16}/> Choose image</button></div>
    {!source ? <div className="panel empty"><h3>Start with an image.</h3><p>Crop, resize, blur or dim a copy for your desktop.</p><button className="primary" disabled={busy} onClick={choose}>Choose image</button></div> : <><div className="panel editor-panel" onChange={() => setEdited("")}><strong title={source}>{source.split(/[\\/]/).pop()}</strong><small>{source}</small>{preview && <img className="edited-preview" src={preview} alt="Preview of the edited image"/>}
      {busy && <div className="editor-loading" role="status"><span className="editor-spinner" aria-hidden="true"/><div><strong>Creating preview…</strong><p>This may take a moment for large images.</p></div></div>}
      <fieldset className="editor-controls" disabled={busy} aria-busy={busy}>
      <div className="editor-fields"><label>Width (px)<input type="number" min="1" max="8192" value={width} onChange={e => setWidth(e.target.value)} placeholder="Original"/></label><label>Height (px)<input type="number" min="1" max="8192" value={height} onChange={e => setHeight(e.target.value)} placeholder="Original"/></label><label>Blur ({blur})<input type="range" min="0" max="20" step="1" value={blur} onChange={e => setBlur(Number(e.target.value))}/></label><label>Dim ({dim}%)<input type="range" min="0" max="90" step="5" value={dim} onChange={e => setDim(Number(e.target.value))}/></label></div>
      <label className="inline-check"><input type="checkbox" checked={crop} onChange={e => setCrop(e.target.checked)}/> Crop image before resizing</label>{crop && <div className="editor-fields">{[["Left (px)", x, setX], ["Top (px)", y, setY], ["Crop width", cropWidth, setCropWidth], ["Crop height", cropHeight, setCropHeight]].map(([label, value, setter]) => <label key={label as string}>{label as string}<input type="number" min="0" value={value as string} onChange={e => (setter as (s: string) => void)(e.target.value)}/></label>)}</div>}
      </fieldset>
      <div className="profile-create"><button className="primary" disabled={busy} onClick={generate}>{busy ? "Creating preview…" : "Create preview"}</button><button className="secondary" disabled={busy || !edited} onClick={async () => { try { await invoke("apply_edited", { path: edited }); setNotice("Edited image applied to your desktop."); } catch (e) { setError(String(e)); } }}>Set as wallpaper</button></div>
      <small>Edits are written to a local cache; the original is never changed.</small></div></>}
  </>;
}

export function HotkeySettings({ hotkeys, refresh }: { hotkeys: Hotkeys; refresh: () => Promise<void> }) {
  const [draft, setDraft] = useState(hotkeys); const [busy, setBusy] = useState(false); const { setError, setNotice } = useSnackbar("settings");
  useEffect(() => setDraft(hotkeys), [hotkeys]);
  useEffect(() => { invoke<string | null>("hotkey_error").then(value => { if (value) setError(value); }).catch(() => {}); }, [setError]);
  async function apply() { setBusy(true); setError(""); setNotice(""); try { await invoke("set_hotkeys", { hotkeys: draft }); await refresh(); setNotice("Shortcuts saved."); } catch (e) { setError(String(e)); } finally { setBusy(false); } }
  return <><div className="section-title utility-heading"><div><span className="eyebrow">SHORTCUTS</span><h2>Control from anywhere</h2></div></div>
    <div className="panel editor-panel"><small>Enter combinations such as Ctrl+Alt+Right. Leave a field empty to disable it.</small><div className="editor-fields">{([ ["Next wallpaper", "next"], ["Previous wallpaper", "previous"], ["Pause or resume", "pause"] ] as const).map(([label, key]) => <label key={key}>{label}<input type="text" maxLength={80} value={draft[key]} placeholder="Not assigned" onChange={e => setDraft({ ...draft, [key]: e.target.value })}/></label>)}</div><button className="secondary" disabled={busy} onClick={apply}>Save shortcuts</button></div>
  </>;
}
