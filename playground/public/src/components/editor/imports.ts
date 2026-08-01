export function importSpecifierAt(source: string, offset: number): string | null {
  const lineStart = source.lastIndexOf("\n", Math.max(0, offset - 1)) + 1;
  const lineEnd = source.indexOf("\n", offset);
  const line = source.slice(lineStart, lineEnd < 0 ? source.length : lineEnd);
  const pattern = /\b(?:from|import)\s*(["'])([^"']+)\1/g;

  for (const match of line.matchAll(pattern)) {
    const specifier = match[2];
    const matchStart = lineStart + (match.index ?? 0);
    const specifierStart = matchStart + match[0].lastIndexOf(specifier);
    const specifierEnd = specifierStart + specifier.length;
    if (offset >= specifierStart && offset <= specifierEnd) return specifier;
  }
  return null;
}

export function resolveWorkspaceImport(fromName: string, specifier: string): string | null {
  if (!specifier.startsWith(".")) return null;
  const base = fromName.split("/").slice(0, -1);
  const segments = [...base, ...specifier.split("/")];
  const resolved: string[] = [];

  for (const segment of segments) {
    if (!segment || segment === ".") continue;
    if (segment === "..") {
      if (resolved.length === 0) return null;
      resolved.pop();
      continue;
    }
    resolved.push(segment);
  }
  return resolved.join("/") || null;
}
