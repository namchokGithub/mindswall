import { useEffect, useRef, useState } from "react";

export type LoadedImage = { src: string; loading: boolean; failed: boolean };

/**
 * Load image bytes into an object URL whenever `key` changes. The previous
 * image stays until the new one is decoded, so a changing wallpaper does not
 * flash an empty placeholder; `loading` is true meanwhile. `load` receives a
 * check that turns false once its result is no longer needed. Without
 * `load`, there is no image.
 */
export function useLoadedImage(key: string, load: ((isActive: () => boolean) => Promise<ArrayBuffer>) | null, type?: string): LoadedImage {
  const [state, setState] = useState<LoadedImage>({ src: "", loading: !!load, failed: false });
  const url = useRef("");
  const replace = (next: string) => { if (url.current) URL.revokeObjectURL(url.current); url.current = next; };
  useEffect(() => () => replace(""), []);
  useEffect(() => {
    if (!load) { replace(""); setState({ src: "", loading: false, failed: true }); return; }
    let active = true;
    setState(previous => ({ ...previous, loading: true, failed: false }));
    load(() => active).then(async bytes => {
      if (!active) return;
      const next = URL.createObjectURL(new Blob([new Uint8Array(bytes)], type ? { type } : undefined));
      // Decode first so the swap is immediate.
      const image = new Image(); image.src = next;
      await image.decode().catch(() => {});
      if (!active) { URL.revokeObjectURL(next); return; }
      replace(next);
      setState({ src: next, loading: false, failed: false });
    }).catch(() => {
      if (!active) return;
      replace("");
      setState({ src: "", loading: false, failed: true });
    });
    return () => { active = false; };
    // `key` identifies what `load` reads, so `load` itself is not a dependency.
  }, [key]);
  return state;
}
