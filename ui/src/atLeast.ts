/** How long a refresh shows its loading state at least, so a fast one is still seen. */
export const MIN_REFRESH_MS = 500;

/**
 * Wait for `work`, and for at least `ms` milliseconds in total. Errors from
 * `work` are passed on after the wait.
 */
export async function atLeast<T>(work: Promise<T>, ms = MIN_REFRESH_MS): Promise<T> {
  const wait = new Promise(resolve => setTimeout(resolve, ms));
  const [result] = await Promise.allSettled([work, wait]);
  if (result.status === "rejected") throw result.reason;
  return result.value;
}
