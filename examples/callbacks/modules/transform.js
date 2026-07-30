import { validate } from "utils";

function capitalize(str) {
  validate(str, "string");
  if (str.length === 0) return str;
  return str.charAt(0).toUpperCase() + str.slice(1);
}

function reverse(str) {
  validate(str, "string");
  let result = "";
  for (let i = str.length - 1; i >= 0; i--) {
    result = result + str.charAt(i);
  }
  return result;
}

function repeat(str, times) {
  validate(str, "string");
  validate(times, "number");
  if (times < 0) throw new Error("repeat: times must be non-negative");
  let result = "";
  for (let i = 0; i < times; i++) {
    result = result + str;
  }
  return result;
}

function slugify(str) {
  validate(str, "string");
  let result = "";
  for (let i = 0; i < str.length; i++) {
    const ch = str.charAt(i);
    const code = ch.charCodeAt(0);
    if ((code >= 97 && code <= 122) || (code >= 48 && code <= 57)) {
      result = result + ch;
    } else if (code >= 65 && code <= 90) {
      result = result + String.fromCharCode(code + 32);
    } else if (ch === " " || ch === "-" || ch === "_") {
      result = result + "-";
    }
  }
  let cleaned = "";
  let prevDash = false;
  for (let i = 0; i < result.length; i++) {
    if (result.charAt(i) === "-") {
      if (!prevDash && cleaned.length > 0) {
        cleaned = cleaned + "-";
      }
      prevDash = true;
    } else {
      cleaned = cleaned + result.charAt(i);
      prevDash = false;
    }
  }
  if (cleaned.length > 0 && cleaned.charAt(cleaned.length - 1) === "-") {
    cleaned = cleaned.slice(0, cleaned.length - 1);
  }
  return cleaned;
}

function wordCount(str) {
  validate(str, "string");
  if (str.length === 0) return 0;
  let count = 0;
  let inWord = false;
  for (let i = 0; i < str.length; i++) {
    const ch = str.charAt(i);
    if (ch === " " || ch === "\t" || ch === "\n") {
      inWord = false;
    } else if (!inWord) {
      count = count + 1;
      inWord = true;
    }
  }
  return count;
}

export { capitalize, reverse, repeat, slugify, wordCount };
