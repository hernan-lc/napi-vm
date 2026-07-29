use crate::error::VmErr;
use crate::value::Value;

/// Bridge that lets the VM call back into host (Node.js) functions.
///
/// The interpreter is single-threaded (`Rc`/`RefCell`, not `Send`/`Sync`), so
/// the bridge is stored as a plain `Rc<dyn HostBridge>` and invoked on the same
/// thread that drives the VM. The concrete implementation lives in
/// `bindings.rs`: it marshals `Value`s across the N-API boundary and calls the
/// persisted JavaScript function synchronously.
pub trait HostBridge {
    /// Invoke the host function registered under `id` with `args`, returning
    /// the marshalled result back into the VM.
    fn call_host(&self, id: usize, args: Vec<Value>) -> Result<Value, VmErr>;
}
