/**
 * The `napi:timers` capability: the host's clock.
 *
 * The VM has its own timer queue with no wall clock — `setTimeout` there
 * orders callbacks without letting guest code observe or wait on real time.
 * That is deliberate, and this capability is the opposite choice, granted
 * explicitly: a plugin that legitimately needs to measure elapsed time or
 * stamp a record asks for `timers`, and a host that does not want its clock
 * fingerprinted withholds it.
 */

import { PermissionDeniedError } from "./errors";
import type { Vm } from "../index";

export const TIMERS_GLOBALS = ["__cap_timers_now", "__cap_timers_hrtime"] as const;

export const TIMERS_MODULE_NAME = "napi:timers";

const TIMERS_MODULE_SOURCE = `
export function now() {
  return __cap_timers_now();
}

export function monotonic() {
  return __cap_timers_hrtime();
}

export function since(start) {
  return __cap_timers_hrtime() - start;
}
`;

export interface TimersCapabilityOptions {
  /**
   * Round the clock to this many milliseconds before the guest sees it.
   *
   * A coarse clock is what makes timing side channels expensive to use, so a
   * host that grants time at all can still deny *precise* time. `0` (the
   * default) does not round.
   */
  resolutionMs?: number;
}

/** Expose the host clock and register `napi:timers`. */
export function installTimersCapability(
  vm: Vm,
  options: TimersCapabilityOptions = {},
): void {
  const resolution = options.resolutionMs ?? 0;
  if (!Number.isFinite(resolution) || resolution < 0) {
    throw new PermissionDeniedError("timer resolution must be a non-negative number");
  }
  const coarsen = (value: number) =>
    resolution > 0 ? Math.floor(value / resolution) * resolution : value;

  vm.exposeFunction("__cap_timers_now", () => coarsen(Date.now()));
  vm.exposeFunction("__cap_timers_hrtime", () => coarsen(performance.now()));

  vm.registerModule(TIMERS_MODULE_NAME, TIMERS_MODULE_SOURCE);
}

/** Remove everything `installTimersCapability` added. */
export function uninstallTimersCapability(vm: Vm): void {
  vm.removeModule(TIMERS_MODULE_NAME);
  for (const name of TIMERS_GLOBALS) vm.removeGlobal(name);
}
