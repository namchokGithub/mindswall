/** What the setup window asks of the setup program. Inside the setup program
 * it calls the Rust commands; in a browser it uses a mock so the screens can
 * be reviewed. Mock options in the address: `?mode=uninstall`,
 * `?installed=0.9.0` (an older version is installed), `?running=1` (MindsWall
 * is open), `?fail=1` (the work fails halfway). */

export type Mode = "install" | "uninstall";

export interface SetupInfo {
  mode: Mode;
  /** The version this setup installs. */
  version: string;
  /** Where to install when nothing is installed yet. */
  defaultLocation: string;
  /** The installed copy, if any. */
  installed: { version: string; location: string; startWithWindows: boolean } | null;
  /** Whether MindsWall is running and must be closed first. */
  appRunning: boolean;
}

export interface InstallOptions {
  location: string;
  desktopShortcut: boolean;
  launchAfter: boolean;
  startWithWindows: boolean;
}

export interface UninstallOptions {
  /** Also delete settings, history, logs, and caches. */
  deleteData: boolean;
}

export interface Progress {
  percent: number;
  step: string;
}

export interface Backend {
  info(): Promise<SetupInfo>;
  /** Ask for a folder; `null` when cancelled. */
  browse(current: string): Promise<string | null>;
  /** Ask the running MindsWall to quit. */
  closeApp(): Promise<void>;
  install(options: InstallOptions, progress: (p: Progress) => void): Promise<void>;
  uninstall(options: UninstallOptions, progress: (p: Progress) => void): Promise<void>;
  launch(): Promise<void>;
  minimize(): void;
  close(): void;
}

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function tauriBackend(): Backend {
  const core = import("@tauri-apps/api/core");
  const win = import("@tauri-apps/api/window");
  const withProgress = async (command: string, args: Record<string, unknown>, progress: (p: Progress) => void) => {
    const { invoke, Channel } = await core;
    const channel = new Channel<Progress>();
    channel.onmessage = progress;
    await invoke(command, { ...args, progress: channel });
  };
  return {
    info: async () => (await core).invoke<SetupInfo>("setup_info"),
    browse: async current => (await core).invoke<string | null>("browse_folder", { current }),
    closeApp: async () => (await core).invoke("close_app"),
    install: (options, progress) => withProgress("install", { options }, progress),
    uninstall: (options, progress) => withProgress("uninstall", { options }, progress),
    launch: async () => (await core).invoke("launch_app"),
    minimize: () => { win.then(({ getCurrentWindow }) => getCurrentWindow().minimize()).catch(() => {}); },
    close: () => { win.then(({ getCurrentWindow }) => getCurrentWindow().close()).catch(() => {}); },
  };
}

function mockBackend(): Backend {
  const query = new URLSearchParams(window.location.search);
  let running = query.get("running") === "1";
  const fail = query.get("fail") === "1";
  const installedVersion = query.get("installed");
  const mode: Mode = query.get("mode") === "uninstall" ? "uninstall" : "install";
  const location = "C:\\Users\\User\\AppData\\Local\\Programs\\MindsWall";
  const wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
  const run = async (steps: string[], progress: (p: Progress) => void) => {
    for (const [index, step] of steps.entries()) {
      progress({ percent: Math.round((index / steps.length) * 100), step });
      await wait(700);
      if (fail && index === Math.floor(steps.length / 2)) {
        throw new Error("Could not write MindsWall.exe: the file is in use by another program.");
      }
    }
    progress({ percent: 100, step: "Done" });
  };
  return {
    info: async () => {
      await wait(300);
      return {
        mode,
        version: "1.0.0",
        defaultLocation: location,
        installed: installedVersion || mode === "uninstall" ? { version: installedVersion ?? "1.0.0", location, startWithWindows: true } : null,
        appRunning: running,
      };
    },
    browse: async current => { await wait(200); return current.endsWith("MindsWall") ? "D:\\Apps\\MindsWall" : current; },
    closeApp: async () => { await wait(900); running = false; },
    install: (_, progress) => run(["Checking WebView2", "Copying MindsWall.exe", "Creating shortcuts", "Registering with Windows"], progress),
    uninstall: (options, progress) => run([
      "Closing MindsWall", "Removing shortcuts", "Removing MindsWall.exe",
      ...(options.deleteData ? ["Deleting settings and caches"] : []), "Removing the Windows entry",
    ], progress),
    launch: async () => { await wait(200); },
    minimize: () => {},
    close: () => { document.body.dataset.closed = "true"; },
  };
}

export const backend: Backend = isTauri ? tauriBackend() : mockBackend();
