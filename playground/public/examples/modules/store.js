import { clamp } from "math";

export class Store {
  constructor(initial) {
    this.state = initial;
    this.listeners = [];
  }
  read(key) { return this.state[key]; }
  write(key, value) {
    this.state[key] = clamp(value, -1000000, 1000000);
    for (let i = 0; i < this.listeners.length; i++) this.listeners[i](key, value);
    return this;
  }
  subscribe(fn) { this.listeners.push(fn); return this; }
}

export function createStore(initial) { return new Store(initial); }
export default createStore;
