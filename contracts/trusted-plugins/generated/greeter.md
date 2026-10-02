# example.greeter 1.0.0

Contract digest: `ca8aa1104513e5324f7aaaca699f3c4160f4b35ba934b4c1fc3d236aa77c9824`

Generated asynchronous, data-only interface. Exact version and digest matching is required.

## Methods

### `greet`

`GreetInput → Promise<GreetOutput>`


Declared domain errors: `INVALID_NAME`

Idempotent metadata: true. Automatic retries are disabled.

## Types

### `GreetInput`


```ts
type GreetInput = { "name": string };
```

### `GreetOutput`


```ts
type GreetOutput = { "message": string };
```

### `InvalidNameData`


```ts
type InvalidNameData = { "reason": string };
```

## Errors and events

- Domain error `INVALID_NAME`: `InvalidNameData`

Generated validators enforce schema constraints not expressible in static types. Optional fields preserve absence; null is accepted only where explicitly declared.
