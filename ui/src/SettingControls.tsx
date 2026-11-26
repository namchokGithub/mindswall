import { Select } from "./Select";

const INTERVALS = [60, 300, 600, 900, 1800, 3600, 21600, 43200, 86400];
const FIT_MODES = ["fill", "fit", "stretch", "center", "tile", "span"];

function formatInterval(seconds: number): string {
  if (seconds % 3600 === 0) {
    const hours = seconds / 3600;
    return `${hours} ${hours === 1 ? "hour" : "hours"}`;
  }
  if (seconds % 60 === 0) {
    const minutes = seconds / 60;
    return `${minutes} ${minutes === 1 ? "minute" : "minutes"}`;
  }
  return `${seconds} seconds`;
}

export function IntervalSelect({ value, disabled, onChange }: { value: number; disabled: boolean; onChange: (value: number) => void }) {
  const values = INTERVALS.includes(value) ? INTERVALS : [value, ...INTERVALS];
  return <Select label="Change picture every" value={value} disabled={disabled} onChange={onChange} options={values.map(seconds => ({ value: seconds, label: formatInterval(seconds) }))}/>;
}

export function FitModeSelect({ value, disabled, onChange }: { value: string; disabled: boolean; onChange: (value: string) => void }) {
  return <Select label="Wallpaper fit mode" value={value} disabled={disabled} onChange={onChange} options={FIT_MODES.map(mode => ({ value: mode, label: mode[0].toUpperCase() + mode.slice(1) }))}/>;
}
