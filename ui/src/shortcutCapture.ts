type KeyStroke = Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "isComposing">;

const supported = new Set([
  "Backquote", "Backslash", "BracketLeft", "BracketRight", "Pause", "Comma", "Equal", "Minus", "Period", "Quote", "Semicolon", "Slash",
  "Backspace", "CapsLock", "Enter", "Space", "Tab", "Delete", "End", "Home", "Insert", "PageDown", "PageUp", "PrintScreen", "ScrollLock",
  "ArrowDown", "ArrowLeft", "ArrowRight", "ArrowUp", "NumLock", "NumpadAdd", "NumpadDecimal", "NumpadDivide", "NumpadEnter", "NumpadEqual", "NumpadMultiply", "NumpadSubtract",
  "AudioVolumeDown", "AudioVolumeUp", "AudioVolumeMute", "MediaPlay", "MediaPause", "MediaPlayPause", "MediaStop", "MediaTrackNext", "MediaTrackPrevious",
]);

export function shortcutFromKey(event: KeyStroke): string | null {
  const code = event.code;
  if (event.isComposing || !(supported.has(code) || /^Key[A-Z]$/.test(code) || /^Digit[0-9]$/.test(code) || /^Numpad[0-9]$/.test(code) || /^F([1-9]|1[0-9]|2[0-4])$/.test(code))) return null;
  return [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Super", code].filter(Boolean).join("+");
}

export function displayShortcut(value: string): string {
  return value.split("+").map(part => part.replace(/^Key([A-Z])$/, "$1").replace(/^Digit([0-9])$/, "$1").replace(/^Arrow(Up|Down|Left|Right)$/, "$1").replace(/^Super$/, "Win")).join("+");
}
