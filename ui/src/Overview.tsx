import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { monitorLabel, useMonitors, type MonitorPlayback } from "./Monitors";
import type { UtilityConfig } from "./Utilities";
import { Spinner } from "./Spinner";
import { useLoadedImage } from "./useLoadedImage";

/** What the main slideshow shows now. */
export type MainPlayback = { current: string; display: string; position: number | null; total: number | null; combined: string[] };

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
const fileName = (path: string) => path.split(/[\\/]/).pop() || path;

/** The full image of a shown wallpaper. Rust only reads images the slideshow shows now. */
function usePreview(path: string) {
  return useLoadedImage(path, path ? () => invoke<ArrayBuffer>("current_preview", { path }) : null);
}

type Card = { key: string; tag: string; label: string; current: string; display: string; position: number | null; total: number | null; combined: string[]; monitor?: string };

function ScreenCard({ card, open, step, busy }: { card: Card; open: () => void; step?: (action: "next" | "previous") => void; busy: boolean }) {
  const { src: preview, loading } = usePreview(card.display || card.current);
  return <div className="preview screen-card">
    {preview && <><div className="screen-card-backdrop" style={{ backgroundImage: `url("${preview}")` }}/><img key={preview} className="screen-card-image loaded-image" src={preview} alt=""/></>}
    {loading && <Spinner size="small" tone="light" overlay label="Loading preview"/>}
    <div className="screen-card-shade"/>
    {card.current && <button type="button" className="preview-open" aria-label={`Show image details for ${card.label}`} title="Show image details" onClick={open}/>}
    {step && <div className="screen-card-actions"><button className="icon-button" disabled={busy} aria-label={`Previous wallpaper on ${card.label}`} title="Previous on this monitor" onClick={() => step("previous")}><ArrowLeft size={15}/></button><button className="icon-button" disabled={busy} aria-label={`Next wallpaper on ${card.label}`} title="Next on this monitor" onClick={() => step("next")}><ArrowRight size={15}/></button></div>}
    <div className="preview-content"><div className="preview-tag"><span className="local-dot"/> {card.tag}</div><h3>{card.current ? fileName(card.current) : "A blank canvas."}{card.combined.length > 0 && <span className="combined-badge">+{card.combined.length} {card.combined.length === 1 ? "image" : "images"}</span>}</h3>{!card.current && !card.monitor && <p>Add a folder to bring your collection to life.</p>}</div>
    <span className="preview-number" aria-label="Image position in slideshow">{card.position !== null && card.total !== null ? `${card.position} / ${card.total}` : "— / —"}</span>
  </div>;
}

/**
 * "Your desktop, right now": one card for the main slideshow while any
 * connected monitor shares it, and one card per connected monitor with its
 * own folders. One card fills the row, two share it, more use three columns.
 */
export function ScreenCards({ config, main, shows, stopped, busy, step, openDetails }: { config: UtilityConfig; main: MainPlayback; shows: MonitorPlayback[]; stopped: boolean; busy: boolean; step: (monitor: string, action: "next" | "previous") => void; openDetails: (path: string, combined: string[]) => void }) {
  const configured = config.monitor_folders.some(entry => entry.folders.some(folder => folder.enabled));
  const { monitors } = useMonitors(configured);
  const label = (id: string) => monitorLabel(id, monitors, config.monitor_names);
  // Until the monitors are listed, assume some monitor shares the main slideshow.
  const sharing = monitors.filter(monitor => !shows.some(show => same(show.monitor, monitor.id)));
  const showMain = shows.length === 0 || monitors.length === 0 || sharing.length > 0;
  const order = (id: string) => { const index = monitors.findIndex(monitor => same(monitor.id, id)); return index < 0 ? monitors.length : index; };
  const own = [...shows].sort((a, b) => order(a.monitor) - order(b.monitor));

  const cards: Card[] = [];
  if (showMain) cards.push({
    key: "main",
    tag: !main.current ? "YOUR NEXT FAVORITE" : shows.length === 0 ? "NOW SHOWING" : `MAIN SLIDESHOW${sharing.length > 0 ? ` · ${sharing.map(monitor => label(monitor.id)).join(", ")}` : ""}`,
    label: "the main slideshow", ...main,
  });
  for (const show of own) cards.push({
    key: show.monitor, tag: `${label(show.monitor)} · OWN FOLDERS`, label: label(show.monitor), monitor: show.monitor,
    current: show.current ?? "", display: show.display ?? "", position: show.position, total: show.total, combined: show.combined ?? [],
  });
  // Span stretches one image over every monitor, so single monitors cannot change.
  const span = config.fit_mode === "span";
  const columns = Math.min(cards.length, 3);

  return <>
    <div className={`screen-cards columns-${columns}`}>{cards.map(card => <ScreenCard key={card.key} card={card} busy={busy}
      open={() => openDetails(card.current, card.combined)}
      step={card.monitor && !span && !stopped ? action => step(card.monitor!, action) : undefined}/>)}</div>
    {own.length > 0 && <p className="footnote screen-cards-note">{span ? "The Span fit mode shows one image across every monitor, so monitors cannot change on their own." : "Point at a monitor's card to change only that monitor. The buttons below change every monitor."}</p>}
  </>;
}
