export type SnackbarOrigin = { scope: string; generation: number };

export function createSnackbarScope(initial: string) {
  let current = initial;
  let generation = 0;
  return {
    capture(scope: string): SnackbarOrigin { return { scope, generation }; },
    accepts(origin: SnackbarOrigin): boolean { return origin.scope === current && origin.generation === generation; },
    navigate(scope: string): boolean {
      if (scope === current) return false;
      current = scope;
      generation += 1;
      return true;
    },
  };
}
