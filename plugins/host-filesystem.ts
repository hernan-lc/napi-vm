import * as fs from "node:fs";

/**
 * The host-side filesystem backend.
 *
 * Everything the plugin host needs from the outside world goes through this
 * interface, so the same permission logic can sit on top of Node, Bun, Deno or
 * a future Rust implementation without the guest-facing API changing.
 *
 * All paths crossing this interface are *native, already-permitted* paths. No
 * implementation of this interface performs permission checks — that happened
 * before the call.
 */
export interface HostFileSystem {
  /**
   * Fully resolved real path (symlinks followed), or `null` when the path does
   * not exist. Any other failure must throw.
   */
  realpath(nativePath: string): string | null;
  readText(nativePath: string): string;
  writeText(nativePath: string, contents: string): void;
  exists(nativePath: string): boolean;
}

/** Node/Bun/Deno-compatible backend built on `node:fs`. */
export function createNodeFileSystem(): HostFileSystem {
  return {
    realpath(nativePath) {
      try {
        return fs.realpathSync(nativePath);
      } catch (error) {
        const code = (error as NodeJS.ErrnoException).code;
        if (code === "ENOENT" || code === "ENOTDIR") return null;
        throw error;
      }
    },
    readText(nativePath) {
      return fs.readFileSync(nativePath, "utf8");
    },
    writeText(nativePath, contents) {
      fs.writeFileSync(nativePath, contents, "utf8");
    },
    exists(nativePath) {
      return fs.existsSync(nativePath);
    },
  };
}
