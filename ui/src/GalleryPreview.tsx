import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";

type Properties = { width: number; height: number; bit_depth: number; size: number; created: number | null; modified: number | null };

function date(value: number | null) {
  return value === null ? "Unavailable" : new Date(value * 1000).toLocaleString();
}
function size(bytes: number) {
  if (bytes < 1024) return `${bytes} bytes`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function GalleryPreview({ path, close }: { path: string; close: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [details, setDetails] = useState<Properties | null>(null);
  const [preview, setPreview] = useState("");
  const [error, setError] = useState("");
  const name = path.split(/[\\/]/).pop() || path;
  const location = path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")));
  const extension = name.includes(".") ? name.split(".").pop()?.toUpperCase() : undefined;
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => { if (element?.open) element.close(); };
  }, []);
  useEffect(() => {
    let active = true; let url = "";
    Promise.all([invoke<Properties>("gallery_details", { path }), invoke<ArrayBuffer>("gallery_preview", { path })]).then(([properties, bytes]) => {
      if (!active) return;
      url = URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
      setPreview(url); setDetails(properties);
    }).catch(e => { if (active) setError(String(e)); });
    return () => { active = false; if (url) URL.revokeObjectURL(url); };
  }, [path]);
  return <dialog ref={dialog} className="gallery-dialog" aria-labelledby="gallery-preview-title" onCancel={event => { event.preventDefault(); close(); }}>
    <header className="gallery-dialog-header"><div><span className="eyebrow">IMAGE DETAILS</span><h2 className="selectable" id="gallery-preview-title">{name}</h2></div><button autoFocus className="icon-button" aria-label="Close preview" onClick={close}><X size={18}/></button></header>
    {error ? <div className="gallery-dialog-error" role="alert"><h3>Could not open this image.</h3><p>{error}</p></div> : !details ? <p className="gallery-dialog-wait" role="status">Loading image details…</p> : <div className="gallery-dialog-content"><div className="gallery-dialog-image"><img src={preview} alt={`Preview of ${name}`}/></div><div className="gallery-properties">
      <section><h3>Image</h3><dl><dt>Dimensions</dt><dd>{details.width} × {details.height}</dd><dt>Width</dt><dd>{details.width} pixels</dd><dt>Height</dt><dd>{details.height} pixels</dd><dt>Bit depth</dt><dd>{details.bit_depth} bits</dd><dt>Resolution (DPI)</dt><dd>Unavailable</dd></dl></section>
      <section><h3>File</h3><dl><dt>Name</dt><dd className="selectable">{name}</dd><dt>Item type</dt><dd>{extension ? `${extension} file` : "Unknown"}</dd><dt>File location</dt><dd className={location ? "selectable" : undefined}>{location || "Unavailable"}</dd><dt>Date created</dt><dd>{date(details.created)}</dd><dt>Date modified</dt><dd>{date(details.modified)}</dd><dt>Size</dt><dd>{size(details.size)}</dd></dl></section>
    </div></div>}
  </dialog>;
}
