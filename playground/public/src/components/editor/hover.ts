export interface HoverInfo {
  label: string;
  detail: string;
}

interface WordRange {
  word: string;
  start: number;
  end: number;
}

const IDENTIFIER = /[\w$]/;

export function hoverAt(source: string, offset: number): HoverInfo | null {
  const range = wordAt(source, offset);
  if (!range) return null;

  const before = source.slice(0, range.start);
  const isProperty = /\.\s*$/.test(before);
  if (isProperty) {
    const type = propertyType(source, range.word);
    return { label: range.word, detail: `(property) ${range.word}: ${type}` };
  }

  const declaration = declarationType(source, range.word);
  if (declaration) {
    return { label: range.word, detail: `${declaration.keyword} ${range.word}: ${declaration.type}` };
  }

  const functionMatch = source.match(new RegExp(`\\bfunction\\s+${escapeRegExp(range.word)}\\s*\\(([^)]*)\\)`));
  if (functionMatch) {
    const params = functionMatch[1].trim();
    return { label: range.word, detail: `(function) ${range.word}(${params})` };
  }

  const builtin = BUILTIN_HOVERS[range.word];
  return builtin ? { label: range.word, detail: builtin } : null;
}

export function wordAt(source: string, offset: number): WordRange | null {
  const cursor = Math.max(0, Math.min(offset, source.length));
  let start = cursor;
  let end = cursor;
  while (start > 0 && IDENTIFIER.test(source[start - 1] || "")) start--;
  while (end < source.length && IDENTIFIER.test(source[end] || "")) end++;
  if (start === end) return null;
  return { word: source.slice(start, end), start, end };
}

function declarationType(source: string, name: string): { keyword: string; type: string } | null {
  const match = source.match(new RegExp(`\\b(const|let|var)\\s+${escapeRegExp(name)}\\s*=\\s*([^;\\n]+)`));
  if (match) return { keyword: match[1], type: expressionType(match[2]) };

  const parameter = new RegExp(`(?:function\\s+\\w*\\s*\\([^)]*\\b${escapeRegExp(name)}\\b|\\([^)]*\\b${escapeRegExp(name)}\\b[^)]*\\)\\s*=>)`);
  if (parameter.test(source)) return { keyword: "parameter", type: parameterType(source, name) };
  return null;
}

function propertyType(source: string, name: string): string {
  const match = source.match(new RegExp(`(?:["']?${escapeRegExp(name)}["']?)\\s*:\\s*([^,}\\n]+)`));
  return match ? expressionType(match[1]) : "unknown";
}

function parameterType(source: string, name: string): string {
  const chain = source.match(new RegExp(`([A-Za-z_$][\\w$]*(?:\\([^\\n]*?\\))?)\\s*\\.then\\s*\\(\\s*\\(\\s*${escapeRegExp(name)}\\s*\\)\\s*=>`));
  if (chain) {
    const functionName = chain[1].replace(/\s*\(.*/, "");
    const returned = functionReturnType(source, functionName);
    if (returned.startsWith("Promise<")) return returned.slice(8, -1);
    if (returned !== "unknown") return returned;
  }

  const call = source.match(new RegExp(`\\b\\w+\\s*\\(\\s*([^,)]+)`));
  if (call && call[1].trim() === name) return expressionType(call[1]);
  return "unknown";
}

function expressionType(expression: string): string {
  const value = expression.trim().replace(/^await\s+/, "");
  if (/^(?:["']|`)/.test(value)) return "string";
  if (/^(?:true|false)\b/.test(value)) return "boolean";
  if (/^-?(?:\d|\.\d)/.test(value)) return "number";
  if (/^\[/.test(value)) return "unknown[]";
  if (/^\{/.test(value)) return "object";
  if (/^(?:Date\.now|Math\.|Number\(|parseInt\(|parseFloat\()/.test(value)) return "number";
  if (/^Promise\.(?:resolve|all|race)\s*\(\s*\{/.test(value)) {
    return value.startsWith("Promise.resolve") && expression.trim().startsWith("await") ? "object" : "Promise<object>";
  }
  if (/^Promise\./.test(value)) return expression.trim().startsWith("await") ? "unknown" : "Promise<unknown>";
  if (/^async\b/.test(value)) return "Promise<unknown>";
  return "unknown";
}

function functionReturnType(source: string, name: string): string {
  const start = source.search(new RegExp(`(?:async\\s+)?function\\s+${escapeRegExp(name)}\\s*\\(`));
  if (start < 0) return "unknown";
  const bodyStart = source.indexOf("{", start);
  if (bodyStart < 0) return "unknown";
  const body = source.slice(bodyStart);
  const returned = body.match(/\breturn\s+([^;\n]+);/);
  if (!returned) return "unknown";

  const returnExpression = returned[1].trim();
  if (/^\w+$/.test(returnExpression)) {
    const declaration = source.match(new RegExp(`\\b(?:const|let|var)\\s+${escapeRegExp(returnExpression)}\\s*=\\s*([^;\\n]+)`));
    if (declaration) return expressionType(declaration[1]);
  }
  const type = expressionType(returnExpression);
  return source.slice(start, bodyStart).trim().startsWith("async") && !type.startsWith("Promise<")
    ? `Promise<${type}>`
    : type;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const BUILTIN_HOVERS: Record<string, string> = {
  console: "var console: Console",
  Date: "var Date: DateConstructor",
  Math: "var Math: Math",
  Promise: "var Promise: PromiseConstructor",
  JSON: "var JSON: JSON",
};
