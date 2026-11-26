import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FolderOpen, Download, Upload, X } from "lucide-react";
import { useSnackbar } from "./Snackbar";
import { useLoadedImage } from "./useLoadedImage";
import { aspectLabel, isStretched, linkedSide, sharedScale, type Size } from "./editorSize";
import { displayShortcut, shortcutFromKey } from "./shortcutCapture";

type Profile = { name: string };
type ExtendedConfig = { profiles: Profile[]; active_profile: string | null };
type Hotkeys = { next: string; previous: string; pause: string };
type EditResult = { path: string; width: number; height: number };
type Props = { config: ExtendedConfig; status: { active: string | null; changed: boolean }; refresh: () => Promise<void> };

export function Profiles({ config, status, refresh }: Props) {
  const [name, setName] = useState(""); const { setError, setNotice } = useSnackbar("profiles"); const [busy, setBusy] = useState(false); const [removing, setRemoving] = useState("");
  async function action(command: string, args: Record<string, unknown>, message: string) {
    setBusy(true); setError(""); setNotice("");
    try { await invoke(command, args); await refresh(); setNotice(message); return true; } catch (e) { await refresh().catch(() => {}); setError(String(e)); return false; } finally { setBusy(false); }
  }
  async function exportFile() {
    setBusy(true); setError(""); setNotice("");
    try { if (await invoke<boolean>("export_backup")) setNotice("Backup exported."); } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }
  async function importFile() {
    setBusy(true); setError(""); setNotice("");
    try { if (await invoke<boolean>("import_backup")) { await refresh(); setNotice("Backup imported."); } } catch (e) { await refresh().catch(() => {}); setError(String(e)); } finally { setBusy(false); }
  }
  return <><div className="section-title"><div><span className="eyebrow">YOUR COLLECTIONS</span><h2>Wallpaper profiles</h2></div></div>
    <div className="panel settings-panel"><div className="setting-row"><div><strong>Save current settings</strong><small>Keep your folders, slideshow and image preferences together.</small></div><div className="profile-create"><input aria-label="Profile name" maxLength={50} value={name} onChange={e => setName([...e.target.value].slice(0, 50).join(""))} placeholder="Profile name (up to 50 characters)"/><button className="primary" disabled={busy || !name.trim()} onClick={async () => { if (await action("save_profile", { name }, "Profile saved.")) setName(""); }}>Save</button></div></div>
    {config.profiles.length === 0 ? <p className="extended-empty">No profiles yet. Save your current setup to get started.</p> : config.profiles.map(profile => { const active = status.active === profile.name; return <div className={`setting-row${active ? " profile-active" : ""}`} key={profile.name}><div><strong>{profile.name}{active && <span className={`profile-badge${status.changed ? " changed" : ""}`}>{status.changed ? "Active · changed" : "Active"}</span>}</strong><small>{active && status.changed ? "Folders or settings changed since this profile was saved." : "Saved wallpaper collection"}</small></div><div className="profile-create">{active && status.changed && <button className="primary" disabled={busy} onClick={() => action("update_profile", { name: profile.name }, "Profile updated.")}>Update profile</button>}<button className="secondary" disabled={busy || (active && !status.changed)} onClick={() => action("apply_profile", { name: profile.name }, "Profile applied.")}>{active ? (status.changed ? "Revert" : "In use") : "Use profile"}</button>{removing === profile.name ? <><button className="secondary" disabled={busy} onClick={async () => { await action("delete_profile", { name: profile.name }, "Profile removed."); setRemoving(""); }}>Confirm remove</button><button className="secondary" onClick={() => setRemoving("")}>Cancel</button></> : <button className="secondary" disabled={busy} onClick={() => setRemoving(profile.name)}>Remove</button>}</div></div>; })}</div>
    <div className="section-title utility-heading"><div><span className="eyebrow">PORTABILITY</span><h2>Backup & restore</h2></div></div><div className="panel setting-row"><div><strong>Your settings, safely stored</strong><small>Export folders, preferences and profiles as JSON. Import replaces current settings and profiles.</small></div><div className="profile-create"><button className="secondary" disabled={busy} onClick={exportFile}><Download size={14}/> Export</button><button className="secondary" disabled={busy} onClick={importFile}><Upload size={14}/> Import</button></div></div>
  </>;
}

export function Editor({ active }: { active: boolean }) {
  const [source, setSource] = useState(""); const [edited, setEdited] = useState(""); const [preview, setPreview] = useState("");
  const [width, setWidth] = useState(""); const [height, setHeight] = useState(""); const [crop, setCrop] = useState(false);
  const [x, setX] = useState("0"); const [y, setY] = useState("0"); const [cropWidth, setCropWidth] = useState(""); const [cropHeight, setCropHeight] = useState("");
  const [blur, setBlur] = useState(0); const [dim, setDim] = useState(0); const { setError, setNotice } = useSnackbar("editor"); const [busy, setBusy] = useState(false);
  const [completion, setCompletion] = useState<{ error?: string; notice?: string } | null>(null);
  const mounted = useRef(false);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => {
    if (!active || !completion) return;
    if (completion.error) setError(completion.error);
    if (completion.notice) setNotice(completion.notice);
    setCompletion(null);
  }, [active, completion, setError, setNotice]);
  useEffect(() => () => { if (preview) URL.revokeObjectURL(preview); }, [preview]);
  // The chosen image, shown as soon as it is picked.
  const original = useLoadedImage(source, source ? () => invoke<ArrayBuffer>("source_preview", { path: source }) : null, "image/png");
  const [size, setSize] = useState<[number, number] | null>(null);
  useEffect(() => {
    setSize(null);
    if (!source) return;
    let active = true;
    invoke<[number, number]>("source_size", { path: source }).then(value => { if (active) setSize(value); }).catch(() => {});
    return () => { active = false; };
  }, [source]);
  const [editedSize, setEditedSize] = useState<Size | null>(null);
  // Resizing keeps the shape of the image, or of the crop, unless turned off.
  const [keepRatio, setKeepRatio] = useState(true);
  const whole = (v: string) => /^\d+$/.test(v) && Number(v) > 0 ? Number(v) : null;
  const cropSize: Size | null = crop && whole(cropWidth) && whole(cropHeight) ? [whole(cropWidth)!, whole(cropHeight)!] : null;
  const base: Size | null = cropSize ?? size;
  const target: Size | null = whole(width) && whole(height) ? [whole(width)!, whole(height)!] : null;
  const stretched = !!base && !!target && isStretched(base, target);
  function changeWidth(value: string) { setWidth(value); if (keepRatio && base) setHeight(linkedSide(value, base, "width")); }
  function changeHeight(value: string) { setHeight(value); if (keepRatio && base) setWidth(linkedSide(value, base, "height")); }
  // Follow a new crop size or the lock being turned on.
  const baseKey = base ? base.join("x") : "";
  useEffect(() => { if (keepRatio && base && width) setHeight(linkedSide(width, base, "width")); }, [baseKey, keepRatio]);
  async function choose() { try { const path = await invoke<string | null>("pick_image"); if (path) { setSource(path); setEdited(""); setPreview(""); setEditedSize(null); setError(""); } } catch (e) { setError(String(e)); } }
  /** Start over: forget the chosen image, the edited preview, and every setting. */
  function clear() {
    setSource(""); setEdited(""); setPreview(""); setEditedSize(null); setKeepRatio(true); setWidth(""); setHeight(""); setCrop(false);
    setX("0"); setY("0"); setCropWidth(""); setCropHeight(""); setBlur(0); setDim(0); setError(""); setNotice("");
  }
  async function generate() {
    setError(""); setNotice(""); setCompletion(null);
    const number = (v: string) => /^\d+$/.test(v) ? Number(v) : null;
    const w = width ? number(width) : null, h = height ? number(height) : null;
    const cropValues = [x, y, cropWidth, cropHeight].map(number);
    if ((!!width !== !!height) || (width && (w === null || h === null)) || (crop && cropValues.some(v => v === null))) { setError("Enter valid whole numbers for both resize dimensions and all crop fields."); return; }
    setEdited(""); setBusy(true);
    try {
      const result = await invoke<EditResult>("edit_image", { path: source, options: { width: w, height: h, crop: crop ? { x: cropValues[0], y: cropValues[1], width: cropValues[2], height: cropValues[3] } : null, blur, dim } });
      const bytes = await invoke<ArrayBuffer>("edited_preview", { path: result.path });
      if (!mounted.current) return;
      setEdited(result.path); setEditedSize([result.width, result.height]); setPreview(URL.createObjectURL(new Blob([bytes], { type: "image/png" }))); setCompletion({ notice: "Preview created. Your original image is unchanged." });
    } catch (e) { if (mounted.current) setCompletion({ error: String(e) }); } finally { if (mounted.current) setBusy(false); }
  }
  return <><div className="section-title"><div><span className="eyebrow">MAKE IT YOURS</span><h2>Image editor</h2></div><div className="profile-create">{source && <button className="secondary" disabled={busy} onClick={clear}><X size={14}/> Clear</button>}<button className="primary" disabled={busy} onClick={choose}><FolderOpen size={16}/> {source ? "Choose another" : "Choose image"}</button></div></div>
    {!source ? <div className="panel empty"><h3>Start with an image.</h3><p>Crop, resize, blur or dim a copy for your desktop.</p><button className="primary" disabled={busy} onClick={choose}>Choose image</button></div> : <><div className="panel editor-panel" onChange={() => setEdited("")}><strong className="selectable">{source.split(/[\\/]/).pop()}</strong><small>{size ? `${size[0]} × ${size[1]} px` : "\u00a0"}</small>
      {(() => {
        // Both previews use one scale, so a smaller edit looks smaller.
        const compare = !!preview && !!editedSize;
        const scale = size ? sharedScale(compare ? [size, editedSize!] : [size]) : null;
        const boxHeight = compare ? 220 : 300;
        const box = scale ? { aspectRatio: `${scale.aspect}`, width: `min(100%, ${Math.round(boxHeight * scale.aspect)}px)` } : undefined;
        return <div className={`editor-previews${compare ? " compare" : ""}`}>
          <figure><div className={`editor-image${scale ? " scaled" : ""}`} style={box}>{original.src ? <img key={original.src} className="loaded-image" src={original.src} style={scale?.images[0]} alt="Preview of the chosen image"/> : <div className="thumbnail-placeholder">{original.failed ? "No preview" : "Loading…"}</div>}{original.loading && original.src && <span className="image-loading" aria-hidden="true"/>}</div><figcaption>Original{size && ` · ${size[0]} × ${size[1]}`}</figcaption></figure>
          {compare && <figure><div className={`editor-image scaled${busy ? " updating" : ""}`} style={box}><img className="loaded-image" src={preview} style={scale?.images[1]} alt="Preview of the edited image"/>{busy && <span className="image-loading" aria-hidden="true"/>}</div><figcaption>Edited · {editedSize![0]} × {editedSize![1]}</figcaption></figure>}
        </div>;
      })()}
      <fieldset className="editor-controls" disabled={busy} aria-busy={busy}>
      <div className="editor-fields"><label>Width (px)<input type="number" min="1" max="8192" value={width} onChange={e => changeWidth(e.target.value)} placeholder="Original"/></label><label>Height (px)<input type="number" min="1" max="8192" value={height} onChange={e => changeHeight(e.target.value)} placeholder="Original"/></label><label>Blur ({blur})<input type="range" min="0" max="20" step="1" value={blur} onChange={e => setBlur(Number(e.target.value))}/></label><label>Dim ({dim}%)<input type="range" min="0" max="90" step="5" value={dim} onChange={e => setDim(Number(e.target.value))}/></label></div>
      <label className="inline-check"><input type="checkbox" checked={keepRatio} onChange={e => setKeepRatio(e.target.checked)}/> Keep aspect ratio{base && ` (${aspectLabel(base)})`}</label>
      {stretched && <p className="editor-warning" role="status">Stretched: the shape changes from {aspectLabel(base!)} to {aspectLabel(target!)}.</p>}
      <label className="inline-check"><input type="checkbox" checked={crop} onChange={e => setCrop(e.target.checked)}/> Crop image before resizing</label>{crop && <div className="editor-fields">{[["Left (px)", x, setX], ["Top (px)", y, setY], ["Crop width", cropWidth, setCropWidth], ["Crop height", cropHeight, setCropHeight]].map(([label, value, setter]) => <label key={label as string}>{label as string}<input type="number" min="0" value={value as string} onChange={e => (setter as (s: string) => void)(e.target.value)}/></label>)}</div>}
      </fieldset>
      <div className="profile-create"><button className="primary" disabled={busy} aria-busy={busy} title="Large images may take a moment." onClick={generate}>{busy && <span className="button-spinner" aria-hidden="true"/>}<span role="status">{busy ? "Creating preview…" : "Create preview"}</span></button><button className="secondary" disabled={busy || !edited} onClick={async () => { try { await invoke("apply_edited", { path: edited }); setNotice("Edited image applied to your desktop."); } catch (e) { setError(String(e)); } }}>Set as wallpaper</button></div>
      <small>Edits are written to a local cache; the original is never changed.</small></div></>}
  </>;
}

export function HotkeySettings({ hotkeys, refresh }: { hotkeys: Hotkeys; refresh: () => Promise<void> }) {
  const [draft, setDraft] = useState(hotkeys); const [busy, setBusy] = useState(false); const { setError, setNotice } = useSnackbar("settings");
  const [recording, setRecording] = useState<keyof Hotkeys | null>(null);
  const [captureHint, setCaptureHint] = useState("");
  useEffect(() => setDraft(hotkeys), [hotkeys]);
  useEffect(() => { invoke<string | null>("hotkey_error").then(value => { if (value) setError(value); }).catch(() => {}); }, [setError]);
  async function apply() { setBusy(true); setError(""); setNotice(""); try { await invoke("set_hotkeys", { hotkeys: draft }); await refresh(); setNotice("Shortcuts saved."); } catch (e) { setError(String(e)); } finally { setBusy(false); } }
  return <><div className="section-title utility-heading"><div><span className="eyebrow">SHORTCUTS</span><h2>Control from anywhere</h2></div></div>
    <div className="panel editor-panel"><small>Select Record, then press a key combination. Press Escape to cancel. Clear disables a shortcut.</small><div className="shortcut-fields">{([ ["Next wallpaper", "next"], ["Previous wallpaper", "previous"], ["Pause or resume", "pause"] ] as const).map(([label, key]) => <div className="shortcut-field" key={key}><span id={`shortcut-${key}`}>{label}</span><div className="shortcut-actions"><button type="button" className="shortcut-record" disabled={busy} aria-labelledby={`shortcut-${key} shortcut-value-${key}`} aria-pressed={recording === key} onClick={() => { setRecording(key); setCaptureHint(""); }} onBlur={() => { setRecording(null); setCaptureHint(""); }} onKeyDown={e => {
      if (recording !== key) return;
      if (e.key === "Tab" && !e.ctrlKey && !e.altKey && !e.metaKey) return;
      e.preventDefault(); e.stopPropagation();
      if (e.key === "Escape") { setRecording(null); setCaptureHint("Recording cancelled."); return; }
      const shortcut = shortcutFromKey(e.nativeEvent);
      if (shortcut) { setDraft(current => ({ ...current, [key]: shortcut })); setRecording(null); setCaptureHint(`${label}: ${displayShortcut(shortcut)} recorded. Save to apply.`); }
      else if (!/^(Control|Shift|Alt|Meta)/.test(e.key)) setCaptureHint("This key cannot be used. Try another key.");
    }}><span id={`shortcut-value-${key}`}>{recording === key ? "Press shortcut…" : `Record: ${displayShortcut(draft[key]) || "Not assigned"}`}</span></button><button type="button" className="secondary" disabled={busy || !draft[key]} aria-label={`Clear ${label} shortcut`} onClick={() => { setDraft(current => ({ ...current, [key]: "" })); setRecording(null); }}>Clear</button></div></div>)}</div><p className="shortcut-hint" role="status">{captureHint || "Some system shortcuts may not be available."}</p><button className="secondary" disabled={busy || recording !== null} onClick={apply}>Save shortcuts</button></div>
  </>;
}
