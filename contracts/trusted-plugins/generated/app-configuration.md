# app.configuration 1.0.0

Contract digest: `db06a9046edec0cff9afe336ca7bea7011133ac037f87dff207f652d2b47113f`

Generated asynchronous, data-only interface. Exact version and digest matching is required.

## Methods

### `get`

`GetInput → Promise<GetOutput>`


Declared domain errors: none

Idempotent metadata: false. Automatic retries are disabled.

## Types

### `GetInput`


```ts
type GetInput = { "key": string };
```

### `GetOutput`


```ts
type GetOutput = { "value": string };
```

## Errors and events

No domain errors or events declared.

Generated validators enforce schema constraints not expressible in static types. Optional fields preserve absence; null is accepted only where explicitly declared.
