import { useEffect, useState, type ReactNode } from "react";
import { AlertTriangle, Check, FolderOpen, Minus, X } from "lucide-react";
import logo from "./assets/logo.png";
import { backend, type InstallOptions, type Progress, type SetupInfo } from "./backend";
import { isAbsoluteWindowsPath, needsAdmin } from "./paths";

type Page = "welcome" | "settings" | "working" | "done" | "error";

export function App() {
  const [info, setInfo] = useState<SetupInfo | null>(null);
  const [loadError, setLoadError] = useState("");
  useEffect(() => { backend.info().then(setInfo).catch(e => setLoadError(String(e))); }, []);
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const uninstall = info?.mode === "uninstall";
  const flow: FlowProps = { setStep, setBusy };
  return <div className="setup">
    <Sidebar info={info} step={step}/>
    <main className="pane">
      <WindowButtons busy={busy}/>
      {loadError ? <Failure title="Setup could not start" message={loadError} close={backend.close}/>
        : !info ? <div className="pane-loading" aria-busy="true"><span className="spinner"/>Getting ready…</div>
        : uninstall ? <UninstallFlow info={info} {...flow}/> : <InstallFlow info={info} {...flow}/>}
    </main>
  </div>;
}

/** How a flow tells the window which step it is on and whether it may close. */
interface FlowProps { setStep: (step: number) => void; setBusy: (busy: boolean) => void }

function Sidebar({ info, step: active }: { info: SetupInfo | null; step: number }) {
  const uninstall = info?.mode === "uninstall";
  const steps = uninstall ? ["Uninstall", "Complete"] : ["Welcome", "Installation", "Complete"];
  return <aside className="side" data-tauri-drag-region>
    <div className="side-brand" data-tauri-drag-region><img src={logo} alt="" draggable={false}/><span data-tauri-drag-region>MindsWall {uninstall ? "Uninstall" : "Setup"}</span></div>
    <div className="side-hero" data-tauri-drag-region>
      <div className="hero-logo" data-tauri-drag-region><img src={logo} alt="" draggable={false}/></div>
      <strong data-tauri-drag-region>MindsWall</strong>
      <span data-tauri-drag-region>Make your desktop yours.</span>
    </div>
    <ol className="side-steps" aria-label="Setup steps">
      {steps.map((step, index) => <li key={step} className={index < active ? "done" : index === active ? "active" : ""} aria-current={index === active ? "step" : undefined}>
        <span className="step-mark">{index < active ? <Check size={13} strokeWidth={3}/> : index + 1}</span>{step}
      </li>)}
    </ol>
  </aside>;
}

function WindowButtons({ busy }: { busy: boolean }) {
  return <div className="window-buttons" data-tauri-drag-region>
    <button aria-label="Minimize" title="Minimize" onClick={backend.minimize}><Minus size={16}/></button>
    <button className="window-close" aria-label="Close" title={busy ? "Wait until setup finishes" : "Close"} disabled={busy} onClick={backend.close}><X size={16}/></button>
  </div>;
}

function Header({ title, subtitle }: { title: string; subtitle: string }) {
  return <header className="pane-header" data-tauri-drag-region><h1 data-tauri-drag-region>{title}</h1><p data-tauri-drag-region>{subtitle}</p></header>;
}

function Footer({ note, children }: { note: string; children: ReactNode }) {
  return <footer className="pane-footer"><span>{note}</span><div className="pane-actions">{children}</div></footer>;
}

function Checkbox({ checked, onChange, label, hint, disabled }: { checked: boolean; onChange: (value: boolean) => void; label: string; hint?: string; disabled?: boolean }) {
  return <label className="check">
    <input type="checkbox" checked={checked} disabled={disabled} onChange={event => onChange(event.target.checked)}/>
    <span className="check-box" aria-hidden="true"><Check size={14} strokeWidth={3}/></span>
    <span className="check-text">{label}{hint && <small>{hint}</small>}</span>
  </label>;
}

function Working({ title, progress }: { title: string; progress: Progress }) {
  return <>
    <Header title={title} subtitle="This takes a few seconds. Please keep this window open."/>
    <div className="pane-body">
      <div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={progress.percent} aria-label={title}><span style={{ width: `${progress.percent}%` }}/></div>
      <div className="progress-text"><span>{progress.step}…</span><span>{progress.percent}%</span></div>
    </div>
    <Footer note="Do not turn off your computer."><button className="primary" disabled>Working…</button></Footer>
  </>;
}

function Failure({ title, message, close, retry }: { title: string; message: string; close: () => void; retry?: () => void }) {
  return <>
    <Header title={title} subtitle="Setup stopped before it finished."/>
    <div className="pane-body">
      <div className="notice notice-error" role="alert"><AlertTriangle size={18}/><div><strong>{message}</strong><span>Close MindsWall and any program that may be using its files, then try again.</span></div></div>
    </div>
    <Footer note="Your settings were not changed.">
      <button className="secondary" onClick={close}>Close</button>
      {retry && <button className="primary" autoFocus onClick={retry}>Try again</button>}
    </Footer>
  </>;
}

function RunningNotice({ info }: { info: SetupInfo }) {
  if (!info.appRunning) return null;
  const detail = info.mode === "uninstall" ? "It will be closed before it is removed."
    : "Setup will close it before it continues. Your slideshow resumes the next time it starts.";
  return <div className="notice" role="status"><AlertTriangle size={18}/><div><strong>MindsWall is open.</strong><span>{detail}</span></div></div>;
}

function InstallFlow({ info, setStep, setBusy }: { info: SetupInfo } & FlowProps) {
  const [page, setPage] = useState<Page>("welcome");
  const update = info.installed !== null;
  const [options, setOptions] = useState<InstallOptions>({
    location: info.installed?.location ?? info.defaultLocation,
    desktopShortcut: true, launchAfter: true, startWithWindows: info.installed?.startWithWindows ?? false,
  });
  const [progress, setProgress] = useState<Progress>({ percent: 0, step: "Starting" });
  const [error, setError] = useState("");
  useEffect(() => setStep(page === "welcome" ? 0 : page === "done" ? 3 : 1), [page, setStep]);
  const set = (patch: Partial<InstallOptions>) => setOptions(current => ({ ...current, ...patch }));
  const locationProblem = !options.location.trim() ? "Choose a folder."
    : !isAbsoluteWindowsPath(options.location) ? "Enter a full path, such as C:\\Users\\You\\Apps\\MindsWall."
    : needsAdmin(options.location) ? "This folder needs administrator rights. Choose a folder in your user profile."
    : "";
  async function install() {
    setPage("working"); setBusy(true); setError("");
    try {
      if (info.appRunning) { setProgress({ percent: 0, step: "Closing MindsWall" }); await backend.closeApp(); }
      await backend.install(options, setProgress);
      setPage("done");
    } catch (e) { setError(String(e).replace(/^Error: /, "")); setPage("error"); }
    finally { setBusy(false); }
  }
  async function finish() {
    if (options.launchAfter) await backend.launch().catch(() => {});
    backend.close();
  }
  const note = "Installs for your user only";
  if (page === "welcome") return <>
    <Header title={update ? "Update MindsWall" : "Welcome to MindsWall"} subtitle={update
      ? `Version ${info.installed!.version} is installed. Setup will update it to ${info.version}.`
      : `Setup will install MindsWall ${info.version} on this computer.`}/>
    <div className="pane-body">
      <ul className="facts">
        <li><Check size={15}/>Works offline. Your images never leave this computer.</li>
        <li><Check size={15}/>{update ? "Your settings, profiles, and history are kept." : "Change wallpapers from your own folders, per monitor."}</li>
        <li><Check size={15}/>Lives in the tray and stays light on memory.</li>
      </ul>
      <RunningNotice info={info}/>
    </div>
    <Footer note={note}><button className="secondary" onClick={backend.close}>Cancel</button><button className="primary" autoFocus onClick={() => setPage("settings")}>Next</button></Footer>
  </>;
  if (page === "settings") return <>
    <Header title="Installation settings" subtitle={update ? "Choose what to set up with the update." : "Choose how you want to install MindsWall."}/>
    <div className="pane-body">
      <label className="field-label" htmlFor="location">Installation location</label>
      <div className="location">
        <input id="location" value={options.location} readOnly={update} spellCheck={false} aria-invalid={!!locationProblem} aria-describedby="location-hint" onChange={event => set({ location: event.target.value })}/>
        <button className="secondary" disabled={update} onClick={async () => { const folder = await backend.browse(options.location); if (folder) set({ location: folder }); }}><FolderOpen size={15}/>Browse</button>
      </div>
      <small id="location-hint" className={locationProblem ? "field-hint is-error" : "field-hint"}>{locationProblem || (update ? "Updating keeps the current location." : "About 15 MB. A folder you can write to without administrator rights.")}</small>
      <div className="checks">
        <Checkbox checked={options.desktopShortcut} onChange={desktopShortcut => set({ desktopShortcut })} label="Create desktop shortcut"/>
        <Checkbox checked={options.launchAfter} onChange={launchAfter => set({ launchAfter })} label="Launch MindsWall after installation"/>
        <Checkbox checked={options.startWithWindows} onChange={startWithWindows => set({ startWithWindows })} label="Start MindsWall with Windows" hint="Starts in the tray. You can change this later in Settings."/>
      </div>
    </div>
    <Footer note={note}><button className="secondary" onClick={() => setPage("welcome")}>Back</button><button className="primary" autoFocus disabled={!!locationProblem} onClick={install}>{update ? "Update" : "Install"}</button></Footer>
  </>;
  if (page === "working") return <Working title={update ? "Updating MindsWall" : "Installing MindsWall"} progress={progress}/>;
  if (page === "error") return <Failure title={update ? "Update failed" : "Installation failed"} message={error} close={backend.close} retry={() => setPage("settings")}/>;
  return <>
    <Header title="MindsWall is ready" subtitle={update ? `Updated to version ${info.version}.` : "Find it in the Start menu and in the tray."}/>
    <div className="pane-body">
      <div className="done-mark"><Check size={30} strokeWidth={2.5}/></div>
      <Checkbox checked={options.launchAfter} onChange={launchAfter => set({ launchAfter })} label="Launch MindsWall now"/>
    </div>
    <Footer note={`Installed in ${options.location}`}><button className="primary" autoFocus onClick={finish}>Finish</button></Footer>
  </>;
}

function UninstallFlow({ info, setStep, setBusy }: { info: SetupInfo } & FlowProps) {
  const [page, setPage] = useState<"confirm" | "working" | "done" | "error">("confirm");
  const [deleteData, setDeleteData] = useState(false);
  const [progress, setProgress] = useState<Progress>({ percent: 0, step: "Starting" });
  const [error, setError] = useState("");
  useEffect(() => setStep(page === "done" ? 2 : 0), [page, setStep]);
  async function uninstall() {
    setPage("working"); setBusy(true);
    try { await backend.uninstall({ deleteData }, setProgress); setPage("done"); }
    catch (e) { setError(String(e).replace(/^Error: /, "")); setPage("error"); }
    finally { setBusy(false); }
  }
  const version = info.installed?.version ?? info.version;
  if (page === "confirm") return <>
    <Header title="Uninstall MindsWall" subtitle={`Remove MindsWall ${version} from this computer.`}/>
    <div className="pane-body">
      <ul className="facts">
        <li><Check size={15}/>Removes the app, its shortcuts, and Start with Windows.</li>
        <li><Check size={15}/>Your image folders are never touched.</li>
      </ul>
      <div className="checks">
        <Checkbox checked={deleteData} onChange={setDeleteData} label="Also delete my settings, history, logs, and caches" hint={deleteData ? "Profiles and settings cannot be recovered." : "Keep them to pick up where you left off if you reinstall."}/>
      </div>
      <RunningNotice info={info}/>
    </div>
    <Footer note={info.installed?.location ?? ""}><button className="secondary" autoFocus onClick={backend.close}>Cancel</button><button className="primary danger" onClick={uninstall}>Uninstall</button></Footer>
  </>;
  if (page === "working") return <Working title="Removing MindsWall" progress={progress}/>;
  if (page === "error") return <Failure title="Uninstall failed" message={error} close={backend.close} retry={() => setPage("confirm")}/>;
  return <>
    <Header title="MindsWall was removed" subtitle="Thanks for giving your desktop a little space."/>
    <div className="pane-body">
      <div className="done-mark"><Check size={30} strokeWidth={2.5}/></div>
      <p className="done-text">{deleteData ? "Your settings and caches were deleted." : "Your settings were kept in %APPDATA%\\MindsWall."} The current wallpaper stays until you change it.</p>
    </div>
    <Footer note=""><button className="primary" autoFocus onClick={backend.close}>Close</button></Footer>
  </>;
}
