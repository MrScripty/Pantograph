// Controlled transport and Vite seams only; tests run the actual ImportManager,
// ComponentRegistry, ErrorReporter and ValidationCache implementations.
export const environment: {
  invoke: (command: string, args: unknown) => Promise<unknown>;
  modules: Record<string, () => Promise<unknown>>;
  lookups: number;
} = {
  invoke: async () => ({ valid: true, error: null }),
  modules: {},
  lookups: 0,
};

export function invoke(command: string, args: unknown): Promise<unknown> {
  return environment.invoke(command, args);
}
export function getComponentModules(): typeof environment.modules {
  environment.lookups++;
  return environment.modules;
}
export function refreshGlobModules(): void {}
export function getModuleCount(): number { return Object.keys(environment.modules).length; }
