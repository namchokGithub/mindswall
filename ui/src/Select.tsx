import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { ChevronDown, Check } from "lucide-react";

export type SelectOption<T> = { value: T; label: string };

/** A dropdown drawn in the app's own style instead of the native control.
 * Keyboard: arrows, Home/End, Enter or Space to choose, Escape to close. */
export function Select<T extends string | number>({ value, options, onChange, disabled = false, label, placeholder = "Select…", className = "" }: {
  value: T | null; options: SelectOption<T>[]; onChange: (value: T) => void; disabled?: boolean; label: string; placeholder?: string; className?: string;
}) {
  const id = useId();
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [upward, setUpward] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const selected = options.findIndex(option => option.value === value);

  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent) => { if (!root.current?.contains(event.target as Node)) setOpen(false); };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [open]);
  useEffect(() => { if (disabled) setOpen(false); }, [disabled]);
  useEffect(() => { if (open) document.getElementById(`${id}-${active}`)?.scrollIntoView({ block: "nearest" }); }, [open, active, id]);

  function show() {
    if (disabled || options.length === 0) return;
    const rect = button.current?.getBoundingClientRect();
    const menuHeight = Math.min(264, options.length * 34 + 8);
    setUpward(!!rect && window.innerHeight - rect.bottom < menuHeight + 8 && rect.top > window.innerHeight - rect.bottom);
    setActive(selected >= 0 ? selected : 0);
    setOpen(true);
  }
  function choose(index: number) {
    const option = options[index];
    setOpen(false);
    button.current?.focus();
    if (option && option.value !== value) onChange(option.value);
  }
  function onKeyDown(event: KeyboardEvent) {
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) { event.preventDefault(); show(); }
      return;
    }
    const last = options.length - 1;
    const moves: Record<string, () => number> = { ArrowDown: () => Math.min(last, active + 1), ArrowUp: () => Math.max(0, active - 1), Home: () => 0, End: () => last, PageDown: () => Math.min(last, active + 5), PageUp: () => Math.max(0, active - 5) };
    if (moves[event.key]) { event.preventDefault(); setActive(moves[event.key]()); }
    else if (event.key === "Enter" || event.key === " ") { event.preventDefault(); choose(active); }
    else if (event.key === "Escape") { event.preventDefault(); setOpen(false); }
    else if (event.key === "Tab") setOpen(false);
  }

  return <div className={`select ${className}`} ref={root}>
    <button type="button" ref={button} className="select-button" role="combobox" aria-label={label} aria-haspopup="listbox" aria-expanded={open} aria-controls={`${id}-list`} aria-activedescendant={open ? `${id}-${active}` : undefined} disabled={disabled} onClick={() => open ? setOpen(false) : show()} onKeyDown={onKeyDown}>
      <span className={selected < 0 ? "select-placeholder" : undefined}>{selected >= 0 ? options[selected].label : placeholder}</span><ChevronDown size={15} aria-hidden="true"/>
    </button>
    {open && <ul className={`select-menu${upward ? " upward" : ""}`} id={`${id}-list`} role="listbox" aria-label={label}>
      {options.map((option, index) => <li key={String(option.value)} id={`${id}-${index}`} role="option" aria-selected={index === selected} className={index === active ? "active" : undefined} onMouseEnter={() => setActive(index)} onMouseDown={event => event.preventDefault()} onClick={() => choose(index)}>
        <span>{option.label}</span>{index === selected && <Check size={14} aria-hidden="true"/>}
      </li>)}
    </ul>}
  </div>;
}
