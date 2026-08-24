/**
 * The `napi:fs` capability: a deliberately tiny, fully-checked filesystem API.
 *
 * The guest never sees `node:fs`. It sees three functions, and every one of
 * them resolves and authorizes its path *itself* — registering the module
 * grants nothing on its own.
 */

import type { Vm } from "../index";

import type { HostFileSystem } from "./host-filesystem";
import type { FsPermissionChecker } from "./permissions";

/** Host globals backing `napi:fs`. Names are convention, never security. */
export const FS_GLOBALS = [
  "__cap_fs_readText",
  "__cap_fs_writeText",
  "__cap_fs_exists",
] as const;

export const FS_MODULE_NAME = "napi:fs";

const FS_MODULE_SOURCE = `
export function readText(path) {
  return __cap_fs_readText(path);
}

export function writeText(path, contents) {
  return __cap_fs_writeText(path, contents);
}

export function exists(path) {
  return __cap_fs_exists(path);
}
`;

export interface FsCapabilityOptions {
  checker: FsPermissionChecker;
  fs: HostFileSystem;
}

/** Expose the checked filesystem functions and register `napi:fs`. */
export function installFsCapability(vm: Vm, options: FsCapabilityOptions): void {
  const { checker, fs } = options;

  vm.exposeFunction("__cap_fs_readText", (requestedPath: unknown) => {
    const { native } = checker.resolve(requestedPath, "read");
    return fs.readText(native);
  });

  vm.exposeFunction(
    "__cap_fs_writeText",
    (requestedPath: unknown, contents: unknown) => {
      if (typeof contents !== "string") {
        throw new TypeError("writeText(path, contents): contents must be a string");
      }
      const { native } = checker.resolve(requestedPath, "write");
      fs.writeText(native, contents);
      return true;
    },
  );

  vm.exposeFunction("__cap_fs_exists", (requestedPath: unknown) => {
    const { native } = checker.resolve(requestedPath, "read");
    return fs.exists(native);
  });

  vm.registerModule(FS_MODULE_NAME, FS_MODULE_SOURCE);
}

/** Remove everything `installFsCapability` added. */
export function uninstallFsCapability(vm: Vm): void {
  vm.removeModule(FS_MODULE_NAME);
  for (const name of FS_GLOBALS) vm.removeGlobal(name);
}
