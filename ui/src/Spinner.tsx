/**
 * The one loading indicator used across the app. With `label`, screen
 * readers announce it; without, it is decorative (for example next to
 * button text that already says what is happening).
 */
export function Spinner({ label, size = "medium", tone = "default", overlay = false }: { label?: string; size?: "small" | "medium" | "large"; tone?: "default" | "light" | "dark"; overlay?: boolean }) {
  return <span className={`spinner spinner-${size} spinner-${tone}${overlay ? " spinner-overlay" : ""}`} role={label ? "status" : undefined} aria-label={label} aria-hidden={label ? undefined : true}/>;
}

/** A spinner centered in the space where content will appear. */
export function LoadingBlock({ label }: { label: string }) {
  return <div className="loading-block"><Spinner size="large" label={label}/></div>;
}
