import { test, expect } from "bun:test";

import { compileFsPermission, compilePattern, matchRule } from "../../plugins";

function matches(pattern: string, candidate: string): boolean {
  return matchRule(compilePattern(pattern, "permissions.fs.read"), candidate);
}

// ── normalization ────────────────────────────────────────────────────

test("undefined and false compile to no rules", () => {
  expect(compileFsPermission(undefined, "f")).toEqual([]);
  expect(compileFsPermission(false, "f")).toEqual([]);
});

test("true and \"*\" both compile to the canonical all-rule", () => {
  expect(compileFsPermission(true, "f")).toEqual([{ kind: "all", pattern: "*" }]);
  expect(compileFsPermission("*", "f")).toEqual([{ kind: "all", pattern: "*" }]);
});

test("a string compiles to one rule and an array to several", () => {
  expect(compileFsPermission("./a", "f")).toHaveLength(1);
  expect(compileFsPermission(["./a", "./b/**", "/etc/hosts"], "f")).toHaveLength(3);
  expect(compileFsPermission([], "f")).toEqual([]);
});

test("relative and absolute rules are told apart", () => {
  expect(compileFsPermission("./cache/**", "f")[0]?.kind).toBe("relative");
  expect(compileFsPermission("/usr/share/**", "f")[0]?.kind).toBe("absolute");
});

test("malformed patterns are rejected at compile time", () => {
  expect(() => compilePattern("", "permissions.fs.read")).toThrow(/empty pattern/);
  expect(() => compilePattern("./", "permissions.fs.read")).toThrow(/empty path/);
  expect(() => compilePattern("../outside/**", "permissions.fs.read")).toThrow(
    /escapes the plugin root/,
  );
  expect(() => compilePattern("./a\0b", "permissions.fs.read")).toThrow(/NUL byte/);
});

test("a pattern is normalized before compiling", () => {
  expect(matches("./assets/../config.json", "config.json")).toBe(true);
});

// ── glob semantics ───────────────────────────────────────────────────

test("a literal pattern matches only that path", () => {
  expect(matches("./config.json", "config.json")).toBe(true);
  expect(matches("./config.json", "config.json.bak")).toBe(false);
  expect(matches("./config.json", "sub/config.json")).toBe(false);
});

test("* stays within one segment", () => {
  expect(matches("./assets/*", "assets/a.png")).toBe(true);
  expect(matches("./assets/*", "assets/b.png")).toBe(true);
  expect(matches("./assets/*", "assets/icons/a.png")).toBe(false);
});

test("** crosses directories", () => {
  expect(matches("./assets/**", "assets/a.png")).toBe(true);
  expect(matches("./assets/**", "assets/icons/a.png")).toBe(true);
  expect(matches("./assets/**", "assets")).toBe(true);
  expect(matches("./assets/**", "assetsx/a.png")).toBe(false);
  expect(matches("./assets/**", "config.json")).toBe(false);
});

test("** may appear in the middle of a pattern", () => {
  expect(matches("./**/*.json", "a.json")).toBe(true);
  expect(matches("./**/*.json", "deep/nested/a.json")).toBe(true);
  expect(matches("./**/*.json", "deep/nested/a.txt")).toBe(false);
});

test("./** covers the whole plugin subtree", () => {
  expect(matches("./**", "config.json")).toBe(true);
  expect(matches("./**", "a/b/c/d.txt")).toBe(true);
});

test("a suffix wildcard matches within a segment only", () => {
  expect(matches("./cache/*.json", "cache/x.json")).toBe(true);
  expect(matches("./cache/*.json", "cache/x.txt")).toBe(false);
  expect(matches("./cache/*.json", "cache/deep/x.json")).toBe(false);
});

test("regex metacharacters in a pattern are literal", () => {
  expect(matches("./a.b/c+d.txt", "a.b/c+d.txt")).toBe(true);
  expect(matches("./a.b/c+d.txt", "axb/cd.txt")).toBe(false);
});

test("absolute patterns match absolute candidates", () => {
  expect(matches("/usr/share/fonts/**", "/usr/share/fonts/x/y.ttf")).toBe(true);
  expect(matches("/usr/share/fonts/**", "/usr/share/other/y.ttf")).toBe(false);
  expect(matches("/etc/hosts", "/etc/hosts")).toBe(true);
  expect(matches("/etc/hosts", "/etc/hosts.bak")).toBe(false);
});

test("the all-rule matches anything", () => {
  const rule = compilePattern("*", "f");
  expect(matchRule(rule, "anything/at/all")).toBe(true);
  expect(matchRule(rule, "/etc/passwd")).toBe(true);
});

// ── `..` prefix vs. a real traversal segment ─────────────────────────

test("a filename that merely starts with two dots is not a traversal", () => {
  // `..cache` is an ordinary (if unusual) directory name. Rejecting it because
  // it starts with ".." confuses a prefix with a path segment.
  expect(() => compilePattern("..cache/**", "permissions.fs.read")).not.toThrow();
  expect(() => compilePattern("..data", "permissions.fs.read")).not.toThrow();
  expect(() => compilePattern("./..cache/*.json", "permissions.fs.read")).not.toThrow();

  expect(matches("..cache/**", "..cache/a.json")).toBe(true);
  expect(matches("..data", "..data")).toBe(true);
});

test("a real `..` segment still escapes the root", () => {
  for (const pattern of ["../outside", "..", "../../etc/**", "./../x", "a/../../b"]) {
    expect(() => compilePattern(pattern, "permissions.fs.read")).toThrow(
      /escapes the plugin root/,
    );
  }
});
