# ADR001: Independent trusted process SDKs

New hosts use supervised child processes and loopback TCP. They neither import the VM nor change legacy capability checks, CommonJS exports, browser/Wasm behavior, or Node-API semantics. Tokens identify sessions; checksums identify bytes. Neither is sandboxing or evidence of trust.

Rust owns native executable launch and may manage explicitly declared native service sidecars. The JS host accepts declared JavaScript runtime artifacts only and consumes externally supplied services. There is no automatic dependency installation, runtime retry after execution, native fallback, operation retry, or universal-addon compatibility claim.

One serial business handler per endpoint uses fail-fast admission when already busy, including independent call chains, rather than queuing a possible distributed deadlock. Callers may explicitly decide whether to retry. A timeout does not free an active handler's state-mutating slot.
