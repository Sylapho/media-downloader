import { checkDependencies, type DependencyStatus } from "./dependencies";

/** IPC/state handling stays outside the dependency view. No polling or timers. */
export function createDependencyCheck() {
  let statuses = $state<DependencyStatus[]>([]);
  let checking = $state(false);
  let error = $state("");

  return {
    get statuses() {
      return statuses;
    },
    get checking() {
      return checking;
    },
    get error() {
      return error;
    },
    async check() {
      if (checking) return;
      checking = true;
      error = "";
      try {
        statuses = await checkDependencies();
      } catch {
        statuses = [];
        error =
          "Could not check dependencies. Open the desktop app and try again.";
      } finally {
        checking = false;
      }
    },
  };
}
