import type { ModuleDef } from "./types.ts";
import SAMPLE from "../examples/playground.js?raw";
import MATH from "../examples/modules/math.js?raw";
import FORMAT from "../examples/modules/format.js?raw";
import STORE from "../examples/modules/store.js?raw";

export const MODULES: ModuleDef[] = [
  { name: "math", source: MATH },
  { name: "format", source: FORMAT },
  { name: "store", source: STORE },
];

export { SAMPLE };
export { default as ASYNC_SAMPLE } from "../examples/async.js?raw";
export { default as LOOP_SAMPLE } from "../examples/loop-guard.js?raw";
