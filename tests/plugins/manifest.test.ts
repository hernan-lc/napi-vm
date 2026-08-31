import { test, expect } from "bun:test";

import { parseManifest, validateManifest } from "../../plugins";

function base(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    name: "example-plugin",
    version: "1.0.0",
    apiVersion: 1,
    entry: "./plugin.js",
    ...overrides,
  };
}

test("accepts a well-formed manifest", () => {
  const manifest = validateManifest(
    base({ permissions: { fs: { read: "./**", write: ["./cache/**"] }, path: true } }),
  );
  expect(manifest.name).toBe("example-plugin");
  expect(manifest.entry).toBe("plugin.js");
  expect(manifest.permissions?.fs?.read).toBe("./**");
  expect(manifest.permissions?.path).toBe(true);
});

test("permissions are optional", () => {
  const manifest = validateManifest(base());
  expect(manifest.permissions).toBeUndefined();
});

test("rejects a non-object manifest", () => {
  expect(() => validateManifest([1, 2])).toThrow(/must be a JSON object/);
  expect(() => validateManifest(null)).toThrow(/must be a JSON object/);
});

test("rejects an empty name", () => {
  expect(() => validateManifest(base({ name: "" }))).toThrow(/name must be a non-empty string/);
});

test("rejects a name with path separators", () => {
  expect(() => validateManifest(base({ name: "../evil" }))).toThrow(/name must match/);
});

test("rejects a missing version", () => {
  expect(() => validateManifest(base({ version: undefined }))).toThrow(/version/);
});

test("rejects a non-integer apiVersion", () => {
  expect(() => validateManifest(base({ apiVersion: "1" }))).toThrow(/apiVersion must be an integer/);
  expect(() => validateManifest(base({ apiVersion: 1.5 }))).toThrow(/apiVersion must be an integer/);
});

test("rejects an unsupported apiVersion", () => {
  expect(() => validateManifest(base({ apiVersion: 2 }))).toThrow(/is not supported/);
});

test("rejects an entry outside the plugin directory", () => {
  expect(() => validateManifest(base({ entry: "../../../outside.js" }))).toThrow(
    /inside the plugin directory/,
  );
  expect(() => validateManifest(base({ entry: "/etc/passwd" }))).toThrow(
    /inside the plugin directory/,
  );
});

test("normalizes the entry path", () => {
  expect(validateManifest(base({ entry: "./src/../plugin.js" })).entry).toBe("plugin.js");
  expect(validateManifest(base({ entry: "src\\main.js" })).entry).toBe("src/main.js");
});

test("rejects a numeric fs permission", () => {
  expect(() => validateManifest(base({ permissions: { fs: { read: 123 } } }))).toThrow(
    /permissions.fs.read must be boolean, string, or string\[\]/,
  );
});

test("rejects non-string entries in an fs permission array", () => {
  expect(() => validateManifest(base({ permissions: { fs: { write: ["./a", 5] } } }))).toThrow(
    /array entries must be strings/,
  );
});

test("rejects a non-boolean path permission", () => {
  expect(() => validateManifest(base({ permissions: { path: "yes" } }))).toThrow(
    /permissions.path must be a boolean/,
  );
});

test("rejects non-object permissions", () => {
  expect(() => validateManifest(base({ permissions: [] }))).toThrow(/permissions must be an object/);
  expect(() => validateManifest(base({ permissions: { fs: "*" } }))).toThrow(
    /permissions.fs must be an object/,
  );
});

test("parseManifest reports invalid JSON", () => {
  expect(() => parseManifest("{ nope")).toThrow(/not valid JSON/);
});

test("parseManifest round-trips a valid document", () => {
  const manifest = parseManifest(JSON.stringify(base()));
  expect(manifest.version).toBe("1.0.0");
});

test("an entry whose first segment merely starts with `..` is accepted", () => {
  const manifest = validateManifest({
    name: "p",
    version: "1.0.0",
    apiVersion: 1,
    entry: "./..build/plugin.js",
  });
  expect(manifest.entry).toBe("..build/plugin.js");
});

test("an entry with a real `..` segment is still rejected", () => {
  for (const entry of ["../plugin.js", "..", "a/../../plugin.js"]) {
    expect(() =>
      validateManifest({ name: "p", version: "1.0.0", apiVersion: 1, entry }),
    ).toThrow(/inside the plugin directory/);
  }
});
