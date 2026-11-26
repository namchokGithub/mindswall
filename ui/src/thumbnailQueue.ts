export function createThumbnailQueue(limit: number) {
  const queue: Array<() => void> = [];
  let active = 0;
  function drain() {
    while (active < limit && queue.length > 0) queue.shift()?.();
  }
  return function enqueue<T>(load: () => Promise<T>, isActive: () => boolean): Promise<T> {
    return new Promise((resolve, reject) => {
      queue.push(() => {
        if (!isActive()) { reject(new Error("Thumbnail no longer needed")); return; }
        active++;
        Promise.resolve().then(load).then(resolve, reject).finally(() => { active--; drain(); });
      });
      drain();
    });
  };
}
