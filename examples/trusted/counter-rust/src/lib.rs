//! Counter state is portable decimal data, and no lock crosses an async boundary.
use napi_vm_plugin_sdk::{
    CallContext, Lifecycle, Plugin, PluginFuture, PluginResult, Registry, RpcError,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
pub mod counter {
    include!("../../../../contracts/trusted-plugins/generated/counter.rs");
}
#[derive(Clone, Default)]
pub struct Counter {
    count: Arc<Mutex<i64>>,
}
impl counter::Handler for Counter {
    fn add(
        &self,
        input: counter::AddInput,
        context: CallContext,
    ) -> PluginFuture<'_, counter::Count> {
        Box::pin(async move {
            context.throw_if_cancelled()?;
            let value = {
                let mut n = self.count.lock().unwrap();
                *n = n.checked_add(input.amount).ok_or_else(|| {
                    RpcError::new("INVALID_ARGUMENT", "counter exceeds i64 range")
                })?;
                *n
            };
            let output = counter::Count {
                count: value.to_string(),
            };
            context
                .emit(
                    &counter::contract()?,
                    "changed",
                    serde_json::to_value(&output)?,
                )
                .await?;
            Ok(output)
        })
    }
    fn get(&self, _: counter::Empty, context: CallContext) -> PluginFuture<'_, counter::Count> {
        Box::pin(async move {
            context.throw_if_cancelled()?;
            Ok(counter::Count {
                count: self.count.lock().unwrap().to_string(),
            })
        })
    }
}
impl Lifecycle for Counter {
    fn initialize(
        &self,
        _: Value,
        _: Value,
        snapshot: Value,
        _: CallContext,
    ) -> PluginFuture<'_, ()> {
        Box::pin(async move {
            if !snapshot.is_null() {
                counter::contract()?.snapshot(&snapshot)?;
                let state: counter::Count =
                    napi_vm_plugin_protocol::from_wire_value(snapshot["data"].clone())?;
                *self.count.lock().unwrap() = state
                    .count
                    .parse()
                    .map_err(|_| RpcError::new("STATE_INCOMPATIBLE", "invalid counter state"))?;
            }
            Ok(())
        })
    }
    fn snapshot(&self, _: CallContext) -> PluginFuture<'_, Value> {
        Box::pin(async move {
            Ok(
                json!({"stateVersion":1,"contract":"example.counter.state","data":{"count":self.count.lock().unwrap().to_string()}}),
            )
        })
    }
}
pub fn plugin() -> PluginResult<Plugin> {
    let registry = Registry::new();
    let counter = Counter::default();
    counter::register(&registry, Arc::new(counter.clone()))?;
    Ok(Plugin::new(registry).with_lifecycle(counter))
}
#[cfg(test)]
mod tests {
    use super::*;
    use napi_vm_plugin_sdk::Harness;
    #[tokio::test]
    async fn snapshot_restore_and_events() {
        let h = Harness::new(plugin().unwrap(), Registry::new()).unwrap();
        h.initialize(json!({}), Value::Null).await.unwrap();
        let mut sub = h
            .subscribe(&counter::contract().unwrap(), "changed", 8)
            .unwrap();
        assert_eq!(
            h.invoke(&counter::contract().unwrap(), "add", json!({"amount":12}))
                .await
                .unwrap(),
            json!({"count":"12"})
        );
        assert_eq!(
            sub.recv().await.unwrap().unwrap()["payload"],
            json!({"count":"12"})
        );
        let state = h.snapshot().await.unwrap();
        h.shutdown().await.unwrap();
        let h2 = Harness::new(plugin().unwrap(), Registry::new()).unwrap();
        h2.initialize(json!({}), state).await.unwrap();
        assert_eq!(
            h2.invoke(&counter::contract().unwrap(), "get", json!({}))
                .await
                .unwrap(),
            json!({"count":"12"})
        );
        h2.shutdown().await.unwrap();
    }
}
