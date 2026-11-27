import { useCallback, useEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { Spinner } from "./Spinner";
import { atLeast } from "./atLeast";

/**
 * Run `action` and report whether it is still running. The running state lasts
 * at least half a second, so a fast refresh still shows that it happened.
 */
export function useRefresh(action: () => Promise<unknown>) {
  const [running, setRunning] = useState(false);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const run = useCallback(async () => {
    setRunning(true);
    try { await atLeast(action()); } catch { /* The action reports its own errors. */ } finally { if (mounted.current) setRunning(false); }
  }, [action]);
  return { running, run };
}

/** An icon button that refreshes something and shows the spinner while it runs. */
export function RefreshButton({ label, onRefresh, disabled = false, size = 16 }: { label: string; onRefresh: () => Promise<unknown>; disabled?: boolean; size?: number }) {
  const { running, run } = useRefresh(onRefresh);
  return <button className="icon-button" aria-label={label} title={label} aria-busy={running} disabled={disabled || running} onClick={run}>
    {running ? <Spinner size="medium"/> : <RefreshCw size={size}/>}
  </button>;
}
