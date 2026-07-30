function validate(value, type) {
  if (typeof value !== type) {
    throw new Error(
      `Validation failed: expected ${type}, got ${typeof value}`
    );
  }
}

function clamp(value, min, max) {
  if (value < min) return min;
  if (value > max) return max;
  return value;
}

function typeOf(value) {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  return typeof value;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(`Assertion failed: ${message}`);
  }
}

export { validate, clamp, typeOf, assert };
