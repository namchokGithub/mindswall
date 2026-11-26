/**
 * Hide the user name in a profile path, such as
 * `C:\Users\name\AppData\...` -> `C:\Users\******\AppData\...`, so the path
 * can be shown on screen or in a screenshot. The mask has a fixed length so it
 * does not reveal the length of the name.
 */
export function maskUserName(path: string): string {
  return path.replace(/^((?:\\\\\?\\)?[A-Za-z]:[\\/]Users[\\/])[^\\/]+/i, "$1******");
}
