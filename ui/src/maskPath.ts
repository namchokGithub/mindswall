/**
 * Hide the user name in a profile path, such as
 * `C:\Users\name\AppData\...` -> `C:\Users\******\AppData\...`, so the path
 * can be shown on screen or in a screenshot. The mask has a fixed length so it
 * does not reveal the length of the name.
 */
export function maskUserName(path: string): string {
  return path.replace(/^((?:\\\\\?\\)?[A-Za-z]:[\\/]Users[\\/])[^\\/]+/i, "$1******");
}

/**
 * Show only the last folder of a path, such as
 * `D:\coding\mindswall\wallpapers` -> `******\wallpapers`, so folder
 * names above it stay off screen. A folder directly on a drive, such as
 * `D:\wallpapers`, and a drive root are shown unchanged.
 */
export function maskFolderPath(path: string): string {
  const trimmed = path.replace(/(?<=[^\\/:])[\\/]+$/, "");
  const index = Math.max(trimmed.lastIndexOf("\\"), trimmed.lastIndexOf("/"));
  if (index < 0) return path;
  const parent = trimmed.slice(0, index);
  if (/^(?:\\\\\?\\)?(?:[A-Za-z]:)?$/.test(parent)) return path;
  return `******${trimmed[index]}${trimmed.slice(index + 1)}`;
}
