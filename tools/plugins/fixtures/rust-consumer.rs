use napi_vm_plugin_host::{Host, LoadOptions, ReloadOptions, Status};
use napi_vm_plugin_protocol::Contract;
use serde_json::json;
use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};
#[allow(dead_code)] mod generated_counter { include!("counter.rs"); }
#[allow(dead_code)] mod generated_greeter { include!("greeter.rs"); }
#[allow(dead_code)] mod generated_wire_types { include!("wire-types.rs"); }
#[allow(dead_code)] mod generated_configuration { include!("app-configuration.rs"); }
fn reaped(pid: u32) {
    #[cfg(target_os="linux")] assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists(), "direct child remains unreaped");
    #[cfg(not(target_os="linux"))] let _ = pid;
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let host=Host::default();let callbacks=Arc::new(AtomicUsize::new(0));let calls=callbacks.clone();
    let configuration=Contract::from_value(serde_json::from_str(include_str!("app-configuration.contract.json"))?)?;
    host.register(configuration,"get",move |_,_| {let calls=calls.clone();Box::pin(async move {calls.fetch_add(1,Ordering::SeqCst);Ok(json!({"value":"Hola"}))})})?;
    let greeter=host.load(&args[1],LoadOptions::default()).await?;
    assert_eq!(greeter.status(),Status::Ready);assert!(greeter.metadata().session_id.is_some());
    let contract=greeter.contract("example.greeter")?;
    assert_eq!(greeter.invoke(&contract,"greet",json!({"name":"Ana"})).await?,json!({"message":"Hola, Ana"}));
    assert!(callbacks.load(Ordering::SeqCst)>0);
    let greeter_pid=greeter.pid().unwrap();greeter.shutdown().await?;assert_eq!(greeter.status(),Status::Stopped);reaped(greeter_pid);
    let counter=host.load(&args[2],LoadOptions::default()).await?;let contract=counter.contract("example.counter")?;
    let mut events=counter.subscribe(&contract,"changed",8)?;
    assert_eq!(counter.invoke(&contract,"add",json!({"amount":12})).await?,json!({"count":"12"}));
    assert!(tokio::time::timeout(Duration::from_secs(3),events.recv()).await??.is_some(), "event stream closed before event");
    let session=counter.metadata().session_id;let pid=counter.pid().unwrap();
    host.reload(&counter.instance_id(),ReloadOptions::default()).await?;
    reaped(pid);assert_ne!(counter.metadata().session_id,session);assert!(!counter.retained_snapshot().is_null());
    assert_eq!(counter.invoke(&contract,"get",json!({})).await?,json!({"count":"12"}));
    assert_eq!(counter.invoke(&contract,"add",json!({"amount":3})).await?,json!({"count":"15"}));
    assert!(tokio::time::timeout(Duration::from_secs(3),events.recv()).await??.is_some(), "event stream closed before event");
    let pid=counter.pid().unwrap();host.shutdown().await?;assert_eq!(counter.status(),Status::Stopped);assert!(counter.history().contains(&Status::Draining));reaped(pid);
    assert!(counter.invoke(&contract,"get",json!({})).await.is_err());host.shutdown().await?;
    println!("rust-consumer: READY callback invoke events snapshot reload quiesce shutdown reap passed");Ok(())
}
