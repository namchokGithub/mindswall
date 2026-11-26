import { Copy } from "lucide-react";
import { maskFolderPath } from "./maskPath";

/** A folder path shown with only its last folder; selecting it copies the full path. */
export function PathCopy({ path, copied }: { path: string; copied: (ok: boolean) => void }) {
  async function copy() {
    try { await navigator.clipboard.writeText(path); copied(true); } catch { copied(false); }
  }
  return <button type="button" className="path-copy" title="Copy full path" aria-label={`Copy full path of ${maskFolderPath(path)}`} onClick={copy}><span>{maskFolderPath(path)}</span><Copy size={11}/></button>;
}
