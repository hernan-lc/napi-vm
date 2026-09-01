import { test, expect } from "bun:test";
import { runCode } from "../index.js";

// ---------------------------------------------------------------------------
// Regular expressions: literals, the constructor, `exec`/`test`, and the
// string methods that take a pattern.
// ---------------------------------------------------------------------------

// --- Literals and construction ----------------------------------------------

test("a regex literal tests a subject", () => {
  expect(runCode("/ab+c/.test('abbbc');")).toBe("true");
  expect(runCode("/ab+c/.test('ac');")).toBe("false");
});

test("source and flags are readable", () => {
  expect(runCode("/x/.source;")).toBe("x");
  expect(runCode("/x/gi.flags;")).toBe("gi");
  expect(runCode("String(/a/g);")).toBe("/a/g");
});

test("flag properties", () => {
  expect(runCode("/a/g.global;")).toBe("true");
  expect(runCode("/a/i.ignoreCase;")).toBe("true");
  expect(runCode("/a/m.multiline;")).toBe("true");
  expect(runCode("/a/s.dotAll;")).toBe("true");
  expect(runCode("/a/y.sticky;")).toBe("true");
});

test("typeof RegExp is function", () => {
  expect(runCode("typeof RegExp;")).toBe("function");
});

test("the constructor compiles a pattern", () => {
  expect(runCode("new RegExp('a+', 'g').test('aaa');")).toBe("true");
});

test("the constructor copies a regex", () => {
  expect(runCode("new RegExp(/a/g).flags;")).toBe("g");
});

test("an invalid pattern throws", () => {
  expect(() => runCode("new RegExp('(');")).toThrow();
});

test("division is still division", () => {
  expect(runCode("const a = 6, b = 3; a / b / 1;")).toBe("2");
});

// --- exec and test ----------------------------------------------------------

test("exec returns the match and its groups", () => {
  expect(runCode("'2024-01-15'.match(/(\\d+)-(\\d+)-(\\d+)/)[1];")).toBe("2024");
});

test("exec reports the match index", () => {
  expect(runCode("/b/.exec('abc').index;")).toBe("1");
});

test("an unmatched optional group is undefined", () => {
  expect(runCode("String(/(a)(b)?/.exec('a')[2]);")).toBe("undefined");
});

test("exec returns null when nothing matches", () => {
  expect(runCode("String(/z/.exec('abc'));")).toBe("null");
});

test("a global regex advances lastIndex", () => {
  expect(runCode("const r = /a/g; r.test('aa'); r.lastIndex;")).toBe("1");
});

test("a global exec loop walks every match", () => {
  expect(
    runCode(
      "const r = /(\\d)/g; const o = []; let m; while ((m = r.exec('1a2')) !== null) o.push(m[1]); o.join();",
    ),
  ).toBe("1,2");
});

test("lastIndex is writable", () => {
  expect(runCode("const r = /a/g; r.lastIndex = 1; r.exec('aa').index;")).toBe("1");
});

// --- Named groups -----------------------------------------------------------

test("named groups are exposed", () => {
  expect(
    runCode("'2024-01'.match(/(?<y>\\d{4})-(?<m>\\d{2})/).groups.y;"),
  ).toBe("2024");
});

test("a named backreference", () => {
  expect(runCode("/(?<c>\\w)\\k<c>/.test('aa');")).toBe("true");
});

// --- Syntax coverage --------------------------------------------------------

test("character classes", () => {
  expect(runCode("/^[a-c]+$/.test('abc');")).toBe("true");
  expect(runCode("/^[^a-c]+$/.test('xyz');")).toBe("true");
});

test("shorthand classes", () => {
  expect(runCode("/^\\w+$/.test('hello_1');")).toBe("true");
  expect(runCode("/^\\s$/.test(' ');")).toBe("true");
  expect(runCode("/^\\D$/.test('a');")).toBe("true");
});

test("anchors and word boundaries", () => {
  expect(runCode("/\\bword\\b/.test('a word here');")).toBe("true");
  expect(runCode("/\\Bord/.test('word');")).toBe("true");
});

test("bounded quantifiers", () => {
  expect(runCode("/^\\d{3}-\\d{4}$/.test('555-1234');")).toBe("true");
  expect(runCode("/^a{2,3}$/.test('aa');")).toBe("true");
  expect(runCode("/^a{2,3}$/.test('a');")).toBe("false");
});

test("lazy quantifiers stop early", () => {
  expect(runCode("/<(.+?)>/.exec('<a><b>')[1];")).toBe("a");
});

test("greedy quantifiers take as much as they can", () => {
  expect(runCode("/<(.+)>/.exec('<a><b>')[1];")).toBe("a><b");
});

test("alternation is leftmost-first", () => {
  expect(runCode("/a|ab/.exec('ab')[0];")).toBe("a");
});

test("dotAll lets . cross a newline", () => {
  expect(runCode("/a.c/s.test('a\\nc');")).toBe("true");
  expect(runCode("/a.c/.test('a\\nc');")).toBe("false");
});

test("multiline anchors match at line breaks", () => {
  expect(runCode("/^b/m.test('a\\nb');")).toBe("true");
  expect(runCode("/^b/.test('a\\nb');")).toBe("false");
});

test("case-insensitive matching", () => {
  expect(runCode("/abc/i.test('ABC');")).toBe("true");
});

test("lookahead", () => {
  expect(runCode("/\\d+(?= dollars)/.exec('42 dollars')[0];")).toBe("42");
  expect(runCode("/(?!foo)\\w+/.exec('bar')[0];")).toBe("bar");
});

test("lookbehind", () => {
  expect(runCode("/(?<=\\$)\\d+/.exec('$42')[0];")).toBe("42");
  expect(runCode("/(?<!\\$)\\d+/.exec('a42')[0];")).toBe("42");
});

test("backreferences", () => {
  expect(runCode("/(\\w)\\1/.test('aa');")).toBe("true");
  expect(runCode("/(\\w)\\1/.test('ab');")).toBe("false");
});

test("non-capturing groups do not capture", () => {
  expect(runCode("/(?:a)(b)/.exec('ab')[1];")).toBe("b");
});

test("a sticky regex must match at lastIndex", () => {
  expect(runCode("const r = /a/y; r.lastIndex = 1; r.test('ba');")).toBe("true");
  expect(runCode("const r = /a/y; r.lastIndex = 1; r.test('ab');")).toBe("false");
});

test("catastrophic backtracking is bounded, not hung", () => {
  expect(() => runCode("/(a+)+$/.test('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab');")).toThrow(
    "backtracking budget",
  );
});

// --- String methods ---------------------------------------------------------

test("match with a global pattern lists the matches", () => {
  expect(runCode("'foo bar'.match(/o/g).join('');")).toBe("oo");
});

test("match returns null when nothing matches", () => {
  expect(runCode("String('test'.match(/z/));")).toBe("null");
});

test("matchAll yields full results", () => {
  expect(runCode("[...'aaa'.matchAll(/a/g)].length;")).toBe("3");
});

test("matchAll requires a global pattern", () => {
  expect(() => runCode("[...'aaa'.matchAll(/a/)];")).toThrow();
});

test("search reports the index", () => {
  expect(runCode("'hello'.search(/l/);")).toBe("2");
  expect(runCode("'hello'.search(/z/);")).toBe("-1");
});

test("replace with a global pattern replaces everything", () => {
  expect(runCode("'aaa'.replace(/a/g, 'b');")).toBe("bbb");
});

test("replace without the global flag replaces once", () => {
  expect(runCode("'aaa'.replace(/a/, 'b');")).toBe("baa");
});

test("replace with a function receives the match", () => {
  expect(runCode("'a1b2'.replace(/\\d/g, (m) => '[' + m + ']');")).toBe("a[1]b[2]");
});

test("replacement patterns reorder groups", () => {
  expect(runCode("'John Smith'.replace(/(\\w+) (\\w+)/, '$2 $1');")).toBe("Smith John");
});

test("$& inserts the whole match", () => {
  expect(runCode("'XaX'.replace(/a/, '[$&]');")).toBe("X[a]X");
});

test("$$ inserts a literal dollar", () => {
  expect(runCode("'x'.replace(/x/, '$$');")).toBe("$");
});

test("a literal needle with a function replacement", () => {
  expect(runCode("'Hello'.replace('l', (m) => m.toUpperCase());")).toBe("HeLlo");
});

test("split on a pattern", () => {
  expect(runCode("'a,b;c'.split(/[,;]/).join('|');")).toBe("a|b|c");
});

test("split keeps the separator's capture groups", () => {
  expect(runCode("'one two'.split(/(\\s)/).length;")).toBe("3");
});

test("split on an empty string yields characters", () => {
  expect(runCode("'abc'.split('').join('-');")).toBe("a-b-c");
});

test("replaceAll on a literal still works", () => {
  expect(runCode("'aaa'.replaceAll('a', 'b');")).toBe("bbb");
});

test("replace on a literal is not treated as a pattern", () => {
  expect(runCode("'2+2'.replace('+', '-');")).toBe("2-2");
});
