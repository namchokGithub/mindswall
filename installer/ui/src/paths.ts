/** Whether `path` is a full Windows folder path such as `C:\Apps\MindsWall`. */
export function isAbsoluteWindowsPath(path: string) {
  return /^[A-Za-z]:\\/.test(path.trim()) || /^\\\\[^\\]+\\[^\\]+/.test(path.trim());
}

/** Folders that need administrator rights to write to. */
export function needsAdmin(path: string) {
  return /^[A-Za-z]:\\(Program Files( \(x86\))?|Windows)(\\|$)/i.test(path.trim());
}
