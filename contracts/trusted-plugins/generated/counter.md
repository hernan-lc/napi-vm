# example.counter 1.0.0

Contract digest: `a6cc7c1e4c4ceedaaa1dde5626c052d5320c1b26ee21c63e3d581d4d972fc90f`

Generated asynchronous, data-only interface. Exact version and digest matching is required.

## Methods

### `add`

`AddInput → Promise<Count>`


Declared domain errors: none

Idempotent metadata: false. Automatic retries are disabled.

### `get`

`Empty → Promise<Count>`


Declared domain errors: none

Idempotent metadata: true. Automatic retries are disabled.

## Types

### `AddInput`


```ts
type AddInput = { "amount": number };
```

### `Count`


```ts
type Count = { "count": string };
```

### `Empty`


```ts
type Empty = {  };
```

## Errors and events

- Event `changed`: `Count`

## State

Snapshot contract `example.counter.state`, state version 1, data `Count`.

Generated validators enforce schema constraints not expressible in static types. Optional fields preserve absence; null is accepted only where explicitly declared.
