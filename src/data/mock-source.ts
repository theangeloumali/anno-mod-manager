import type { DataSource } from "./source.ts";

// Foundation stub so the app builds; the mockup-v2 slice replaces it with full mock data.
const notYet = (): Promise<never> =>
  Promise.reject(new Error("mock data not built yet"));

export const mockSource: DataSource = new Proxy({} as DataSource, {
  get: (_target, property) => (property === "then" ? undefined : notYet),
});
