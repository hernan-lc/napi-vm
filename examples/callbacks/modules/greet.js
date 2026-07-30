import { validate } from "utils";

function greet(name) {
  validate(name, "string");
  return `Hello, ${name}! Welcome to the VM.`;
}

function farewell(name) {
  validate(name, "string");
  return `Goodbye, ${name}! See you next time.`;
}

function announce(message, audience) {
  validate(message, "string");
  validate(audience, "string");
  return `[${audience}] ${message}`;
}

export { greet, farewell, announce };
export default greet;
