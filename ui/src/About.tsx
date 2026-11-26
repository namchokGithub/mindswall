import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Bug, Code, Download, ExternalLink, Scale } from "lucide-react";
import { useSnackbar } from "./Snackbar";

type AppInfo = { name: string; version: string; source: string; license: string };
type Link = "source" | "issues" | "releases" | "license";

/** What the app is, how it treats your data, and where its source code lives. */
export function AboutPage() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const { setError } = useSnackbar("about");
  useEffect(() => { invoke<AppInfo>("app_info").then(setInfo).catch(e => setError(String(e))); }, [setError]);
  async function open(link: Link) {
    try { await invoke("open_project_link", { link }); } catch (e) { setError(String(e)); }
  }
  const links: { link: Link; icon: typeof Code; title: string; text: string }[] = [
    { link: "source", icon: Code, title: "Source code", text: "Read the code, build it yourself, or contribute on GitHub." },
    { link: "releases", icon: Download, title: "Releases", text: "Download the installer, the portable ZIP, and checksums." },
    { link: "issues", icon: Bug, title: "Report an issue", text: "Tell us about a bug or suggest an improvement." },
    { link: "license", icon: Scale, title: `${info?.license ?? "MIT"} license`, text: "Free to use, change, and share." },
  ];
  return <>
    <div className="section-title"><div><span className="eyebrow">THE APP</span><h2>{info?.name ?? "MindsWall"} <span className="count-pill">Version {info?.version ?? "…"}</span></h2></div></div>
    <div className="panel about-panel">
      <p>MindsWall is a lightweight wallpaper manager for Windows 11. It shows images from your own folders as a slideshow, on one monitor or each monitor separately, and can combine images to fill the screen.</p>
      <ul className="about-facts">
        <li><strong>Offline by design.</strong> Your images, settings, and history stay on this computer. The app never sends data anywhere; the links below only open your browser when you choose them.</li>
        <li><strong>No administrator rights.</strong> It installs and runs for your account only.</li>
        <li><strong>Your files stay untouched.</strong> Edits and combined wallpapers are copies in a local cache.</li>
      </ul>
    </div>
    <div className="section-title utility-heading"><div><span className="eyebrow">OPEN SOURCE</span><h2>Source code and help</h2></div></div>
    <div className="panel settings-panel">{links.map(({ link, icon: Icon, title, text }) => <div className="setting-row" key={link}><div className="about-link"><div className="summary-icon"><Icon size={17}/></div><div><strong>{title}</strong><small>{text}</small></div></div><button className="secondary about-open" onClick={() => open(link)}>Open <ExternalLink size={13}/></button></div>)}</div>
    {info && <p className="footnote selectable">{info.source}</p>}
  </>;
}
