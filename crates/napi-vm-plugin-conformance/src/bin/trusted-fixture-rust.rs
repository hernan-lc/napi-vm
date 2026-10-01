//! Controlled failure fixture. Never runs arbitrary third-party code.
use napi_vm_plugin_protocol::Cancellation;
use napi_vm_plugin_sdk::*;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
fn contract() -> PluginResult<Contract> {
    Contract::from_value(serde_json::from_str(include_str!(
        "../../../../contracts/trusted-plugins/generated/counter.contract.json"
    ))?)
}
#[derive(Clone, Default)]
struct Hooks {
    mode: Arc<Mutex<String>>,
    count: Arc<Mutex<i64>>,
}
impl Lifecycle for Hooks {
    fn initialize(
        &self,
        config: Value,
        _: Value,
        snapshot: Value,
        context: CallContext,
    ) -> PluginFuture<'_, ()> {
        Box::pin(async move {
            let mode = config["mode"].as_str().unwrap_or("normal").to_string();
            *self.mode.lock().unwrap() = mode.clone();
            if mode == "initialize-error" {
                return Err(RpcError::new(
                    "INTERNAL_ERROR",
                    "controlled initialize failure",
                ));
            }
            if !snapshot.is_null() {
                *self.count.lock().unwrap() =
                    snapshot["data"]["count"].as_str().unwrap().parse().unwrap();
            }
            if mode == "uncooperative-task" {
                context.resources().spawn(|_: Cancellation| {
                    Box::pin(async { std::future::pending::<()>().await })
                })?;
            }
            if mode == "logs" {
                for _ in 0..4096 {
                    println!("controlled noisy stdout abcdefghijklmnopqrstuvwxyz");
                    eprintln!("controlled noisy stderr abcdefghijklmnopqrstuvwxyz");
                }
            }
            Ok(())
        })
    }
    fn snapshot(&self, _: CallContext) -> PluginFuture<'_, Value> {
        Box::pin(async move {
            if *self.mode.lock().unwrap() == "snapshot-error" {
                return Err(RpcError::new(
                    "INTERNAL_ERROR",
                    "controlled snapshot failure",
                ));
            }
            Ok(
                json!({"stateVersion":1,"contract":"example.counter.state","data":{"count":self.count.lock().unwrap().to_string()}}),
            )
        })
    }
    fn shutdown(&self, _: CallContext) -> PluginFuture<'_, ()> {
        Box::pin(async move {
            if *self.mode.lock().unwrap() == "shutdown-error" {
                return Err(RpcError::new(
                    "INTERNAL_ERROR",
                    "controlled shutdown failure",
                ));
            }
            Ok(())
        })
    }
}
#[tokio::main]
async fn main() {
    let result = async {
        let hooks = Hooks::default();
        let registry = Registry::new();
        let h = hooks.clone();
        registry.register(contract()?, "add", move |v, _| {
            let h = h.clone();
            Box::pin(async move {
                let mode = h.mode.lock().unwrap().clone();
                if mode == "exit-call" {
                    std::process::exit(7);
                }
                let n = {
                    let mut n = h.count.lock().unwrap();
                    *n += v["amount"].as_i64().unwrap();
                    *n
                };
                if mode == "ignore-cancel" {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
                Ok(json!({"count":n.to_string()}))
            })
        })?;
        let h = hooks.clone();
        registry.register(contract()?, "get", move |_, _| {
            let h = h.clone();
            Box::pin(async move { Ok(json!({"count":h.count.lock().unwrap().to_string()})) })
        })?;
        serve(
            Plugin::new(registry).with_lifecycle(hooks),
            PluginMetadata {
                id: "example.counter".into(),
                version: "0.1.0".into(),
                requires_host: vec![],
            },
        )
        .await
    }
    .await;
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
