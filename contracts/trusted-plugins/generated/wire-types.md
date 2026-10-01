# example.wire-types 1.0.0

Contract digest: `a0cdc912547e345bc9def20c0bb4110b60908cbcf4178fb68bea1e72734f2a7b`

Generated asynchronous, data-only interface. Exact version and digest matching is required.

## Methods

### `echo`

`WireTypes → Promise<WireTypes>`


Declared domain errors: none

Idempotent metadata: false. Automatic retries are disabled.

## Types

### `WireTypes`


```ts
type WireTypes = { "__proto__": string; "choice": "🌍" | "ready" | "quote\"\\value" | "literal\\u0010" | "controls\b\f"; "created": string; "data": string; "integer": number; "literal": 1; "optionalNull"?: string | null; "optionalNumber"?: number | null; "self"?: string | null; "self-field"?: string; "self_field"?: string; "tagged": ({ "kind": "text"; "value": string }) | ({ "kind": "number"; "value": number }); "wide": string };
```

## Errors and events

No domain errors or events declared.

Generated validators enforce schema constraints not expressible in static types. Optional fields preserve absence; null is accepted only where explicitly declared.
