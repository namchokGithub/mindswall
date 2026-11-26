import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronLeft, ChevronRight, X } from "lucide-react";

type Properties = { width: number; height: number; bit_depth: number; size: number; created: number | null; modified: number | null };

function date(value: number | null) {
  return value === null ? "Unavailable" : new Date(value * 1000).toLocaleString();
}
function size(bytes: number) {
  if (bytes < 1024) return `${bytes} bytes`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Image details. `combined` lists the images shown next to `path` in a
 * combined wallpaper; then the arrows (or the Left and Right keys) move
 * between all of them.
 */
export function GalleryPreview({ path: first, combined = [], close }: { path: string; combined?: string[]; close: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const images = [first, ...combined];
  const [index, setIndex] = useState(0);
  const path = images[Math.min(index, images.length - 1)];
  const [details, setDetails] = useState<Properties | null>(null);
  const [preview, setPreview] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const name = path.split(/[\\/]/).pop() || path;
  const location = path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")));
  const extension = name.includes(".") ? name.split(".").pop()?.toUpperCase() : undefined;
  const many = images.length > 1;
  const step = (offset: number) => setIndex(current => (current + offset + images.length) % images.length);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => { if (element?.open) element.close(); };
  }, []);
  useEffect(() => () => { if (preview) URL.revokeObjectURL(preview); }, [preview]);
  useEffect(() => {
    // The previous image stays visible until the next one is ready.
    let active = true;
    setLoading(true); setError("");
    Promise.all([invoke<Properties>("gallery_details", { path }), invoke<ArrayBuffer>("gallery_preview", { path })]).then(([properties, bytes]) => {
      if (!active) return;
      setPreview(URL.createObjectURL(new Blob([bytes], { type: "image/png" }))); setDetails(properties); setLoading(false);
    }).catch(e => { if (active) { setError(String(e)); setLoading(false); } });
    return () => { active = false; };
  }, [path]);
  const keys = (event: React.KeyboardEvent) => {
    if (!many) return;
    if (event.key === "ArrowLeft") { event.preventDefault(); step(-1); }
    if (event.key === "ArrowRight") { event.preventDefault(); step(1); }
  };
  const nav = many && <div className="gallery-dialog-nav"><button className="icon-button" aria-label="Previous image" title="Previous image (Left)" onClick={() => step(-1)}><ChevronLeft size={18}/></button><span aria-live="polite">{index + 1} / {images.length}</span><button className="icon-button" aria-label="Next image" title="Next image (Right)" onClick={() => step(1)}><ChevronRight size={18}/></button></div>;
  return <dialog ref={dialog} className="gallery-dialog" aria-labelledby="gallery-preview-title" onCancel={event => { event.preventDefault(); close(); }} onKeyDown={keys}>
    <header className="gallery-dialog-header"><div><span className="eyebrow">{many ? "COMBINED WALLPAPER" : "IMAGE DETAILS"}</span><h2 className="selectable" id="gallery-preview-title">{name}</h2></div><div className="profile-create">{nav}<button autoFocus className="icon-button" aria-label="Close preview" onClick={close}><X size={18}/></button></div></header>
    {error ? <div className="gallery-dialog-error" role="alert"><h3>Could not open this image.</h3><p>{error}</p></div> : !details ? <p className="gallery-dialog-wait" role="status">Loading image details…</p> : <div className="gallery-dialog-content"><div className="gallery-dialog-image"><img key={preview} className="loaded-image" src={preview} alt={`Preview of ${name}`}/>{loading && <span className="image-loading" role="status" aria-label="Loading image"/>}</div><div className="gallery-properties">
      <section><h3>Image</h3><dl><dt>Dimensions</dt><dd>{details.width} × {details.height}</dd><dt>Width</dt><dd>{details.width} pixels</dd><dt>Height</dt><dd>{details.height} pixels</dd><dt>Bit depth</dt><dd>{details.bit_depth} bits</dd><dt>Resolution (DPI)</dt><dd>Unavailable</dd></dl></section>
      <section><h3>File</h3><dl><dt>Name</dt><dd className="selectable">{name}</dd><dt>Item type</dt><dd>{extension ? `${extension} file` : "Unknown"}</dd><dt>File location</dt><dd className={location ? "selectable" : undefined}>{location || "Unavailable"}</dd><dt>Date created</dt><dd>{date(details.created)}</dd><dt>Date modified</dt><dd>{date(details.modified)}</dd><dt>Size</dt><dd>{size(details.size)}</dd></dl></section>
      {many && <section><h3>In this wallpaper</h3><ol className="combined-list">{images.map((image, position) => <li key={image}><button className={`combined-item${position === index ? " current" : ""}`} aria-current={position === index ? "true" : undefined} title={image.split(/[\\/]/).pop() || image} onClick={() => setIndex(position)}>{image.split(/[\\/]/).pop() || image}</button></li>)}</ol></section>}
    </div></div>}
  </dialog>;
}
