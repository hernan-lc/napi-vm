# Manifest and dependency inventory

Only `manifestVersion:2, execution:"trusted-process"` is accepted. `id`, implementation `version`, `protocol`, `provides`, `requiresHost`, launch, profile and relative contract artifact paths identify the package. The new format has no application permission field.

JavaScript launch specifies built entry, declared runtimes and preferred runtime. Native launch specifies executable entry plus actual OS/architecture and Linux libc. Host runtime selection occurs once before execution; a plugin failure never triggers a different runtime. TS host rejects executable launch, including native Rust.

Profiles distinguish `portable-js` from `external-runtime`. Native dependencies must declare name/path/target/ABI/runtime and availability. Artifact inventories separately record what is built/distributed/tested; none implies all other targets exist. The existing VM Node-API subset is not universal npm/native-addon compatibility.

Dependencies services identify name and kind `http` or `mcp`, plus owner (`external` by default or explicit `host`). Host-owned services require HTTP readiness path/status and shutdown:`terminate`; the Rust adapter must match these declarations. TS rejects host-owned services. Capabilities identify host-service contract IDs. Runtime service connections are injected through explicit host options. Never write credentials, bootstrap values or tokens into manifests, package locks, documentation examples or logs.

Entry/assets/contracts/native paths are relative to the package. Package integrity excludes traversal, broken or escaping symlinks. This does not confine a running trusted plugin's filesystem access.
