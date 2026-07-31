export interface Token {
  type: "keyword" | "string" | "comment" | "number" | "operator" | "punctuation" | "function" | "property" | "text";
  value: string;
}

const KEYWORDS = new Set([
  "const", "let", "var", "function", "return", "if", "else", "for", "while", "do",
  "switch", "case", "break", "continue", "new", "delete", "typeof", "instanceof",
  "in", "of", "class", "extends", "super", "this", "static", "get", "set",
  "import", "export", "from", "default", "as", "async", "await", "yield",
  "try", "catch", "finally", "throw", "typeof", "void", "null", "undefined",
  "true", "false", "debugger", "with",
]);

export function tokenize(code: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;

  while (i < code.length) {
    // whitespace
    if (/\s/.test(code[i])) {
      let start = i;
      while (i < code.length && /\s/.test(code[i])) i++;
      tokens.push({ type: "text", value: code.slice(start, i) });
      continue;
    }

    // line comment
    if (code[i] === "/" && code[i + 1] === "/") {
      let start = i;
      while (i < code.length && code[i] !== "\n") i++;
      tokens.push({ type: "comment", value: code.slice(start, i) });
      continue;
    }

    // block comment
    if (code[i] === "/" && code[i + 1] === "*") {
      let start = i;
      i += 2;
      while (i < code.length && !(code[i] === "*" && code[i + 1] === "/")) i++;
      if (i < code.length) i += 2;
      tokens.push({ type: "comment", value: code.slice(start, i) });
      continue;
    }

    // string (single or double quote)
    if (code[i] === '"' || code[i] === "'" || code[i] === "`") {
      const quote = code[i];
      let start = i;
      i++;
      while (i < code.length && code[i] !== quote) {
        if (code[i] === "\\") i++;
        i++;
      }
      if (i < code.length) i++;
      tokens.push({ type: "string", value: code.slice(start, i) });
      continue;
    }

    // number
    if (/[0-9]/.test(code[i]) || (code[i] === "." && /[0-9]/.test(code[i + 1]))) {
      let start = i;
      if (code[i] === "0" && (code[i + 1] === "x" || code[i + 1] === "X")) {
        i += 2;
        while (i < code.length && /[0-9a-fA-F]/.test(code[i])) i++;
      } else {
        while (i < code.length && /[0-9.]/.test(code[i])) i++;
      }
      tokens.push({ type: "number", value: code.slice(start, i) });
      continue;
    }

    // identifier / keyword
    if (/[a-zA-Z_$]/.test(code[i])) {
      let start = i;
      while (i < code.length && /[a-zA-Z0-9_$]/.test(code[i])) i++;
      const word = code.slice(start, i);

      if (KEYWORDS.has(word)) {
        tokens.push({ type: "keyword", value: word });
      } else if (i < code.length && code[i] === "(") {
        tokens.push({ type: "function", value: word });
      } else if (start > 0 && code[start - 1] === ".") {
        tokens.push({ type: "property", value: word });
      } else {
        tokens.push({ type: "text", value: word });
      }
      continue;
    }

    // operators
    if (/[+\-*/%=!<>&|^~?:]/.test(code[i])) {
      let start = i;
      i++;
      while (i < code.length && /[+\-*/%=!<>&|^~?:]/.test(code[i])) i++;
      tokens.push({ type: "operator", value: code.slice(start, i) });
      continue;
    }

    // punctuation
    if (/[{}()\[\];,.]/.test(code[i])) {
      tokens.push({ type: "punctuation", value: code[i] });
      i++;
      continue;
    }

    // anything else
    tokens.push({ type: "text", value: code[i] });
    i++;
  }

  return tokens;
}

export function highlightToHtml(code: string): string {
  return tokenize(code)
    .map((tok) => {
      if (tok.type === "text") return escHtml(tok.value);
      return `<span class="tok-${tok.type}">${escHtml(tok.value)}</span>`;
    })
    .join("");
}

function escHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
