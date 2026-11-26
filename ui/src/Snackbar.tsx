import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { CheckCircle2, CircleAlert, X } from "lucide-react";
import { createSnackbarScope } from "./snackbarScope";
import "./snackbar.css";

type Tone = "success" | "error";
type Message = { id: number; text: string; tone: Tone };
type ContextValue = {
  generation: number;
  navigate: (scope: string) => void;
  show: (scope: string, generation: number, text: string, tone: Tone) => void;
};

const SnackbarContext = createContext<ContextValue | null>(null);

export function SnackbarProvider({ children }: { children: ReactNode }) {
  const active = useRef(createSnackbarScope("overview"));
  const nextId = useRef(0);
  const [generation, setGeneration] = useState(0);
  const [message, setMessage] = useState<Message | null>(null);

  const navigate = useCallback((scope: string) => {
    if (!active.current.navigate(scope)) return;
    setGeneration(active.current.capture(scope).generation);
    setMessage(null);
  }, []);
  const show = useCallback((scope: string, origin: number, text: string, tone: Tone) => {
    if (!active.current.accepts({ scope, generation: origin })) return;
    if (text) setMessage({ id: ++nextId.current, text, tone });
    else setMessage(current => current?.tone === tone ? null : current);
  }, []);

  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(() => setMessage(null), message.tone === "error" ? 8000 : 4000);
    return () => window.clearTimeout(timer);
  }, [message]);

  return <SnackbarContext.Provider value={{ generation, navigate, show }}>
    {children}
    <div className="snackbar-region">
      {message && <div className={`snackbar snackbar--${message.tone}`} role={message.tone === "error" ? "alert" : "status"}>
        {message.tone === "error" ? <CircleAlert size={19} aria-hidden="true"/> : <CheckCircle2 size={19} aria-hidden="true"/>}
        <span>{message.text}</span>
        <button type="button" onClick={() => setMessage(null)} aria-label="Dismiss notification"><X size={17}/></button>
      </div>}
    </div>
  </SnackbarContext.Provider>;
}

export function useSnackbar(scope: string) {
  const context = useContext(SnackbarContext);
  if (!context) throw new Error("SnackbarProvider is missing");
  const { show, generation, navigate } = context;
  const setError = useCallback((text: string) => show(scope, generation, text, "error"), [show, scope, generation]);
  const setNotice = useCallback((text: string) => show(scope, generation, text, "success"), [show, scope, generation]);
  return { setError, setNotice, navigate };
}
