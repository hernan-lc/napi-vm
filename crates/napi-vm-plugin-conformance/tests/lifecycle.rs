use napi_vm_plugin_host::*;
use napi_vm_plugin_protocol::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Duration};
struct Package(PathBuf);
impl Drop for Package {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn contract() -> Contract {
    Contract::from_value(
        serde_json::from_str(include_str!(
            "../../../contracts/trusted-plugins/generated/counter.contract.json"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn package() -> Package {
    let root = std::env::temp_dir().join(format!(
        "napi-rust-conformance-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("bin")).unwrap();
    std::fs::create_dir(root.join("contracts")).unwrap();
    let exe = if cfg!(windows) {
        "bin/plugin.exe"
    } else {
        "bin/plugin"
    };
    std::fs::copy(env!("CARGO_BIN_EXE_trusted-fixture-rust"), root.join(exe)).unwrap();
    let manifest = json!({"manifestVersion":2,"execution":"trusted-process","id":"example.counter","version":"0.1.0","protocol":{"major":1,"minMinor":0,"maxMinor":0},"provides":{"example.counter":"1.0.0"},"requiresHost":{},"profile":"native-executable","launch":{"kind":"executable","entry":exe,"args":[],"target":Target::current()},"assets":[],"contracts":["contracts/counter.contract.json"]});
    std::fs::write(
        root.join("plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("contracts/counter.contract.json"),
        serde_json::to_vec(&contract()).unwrap(),
    )
    .unwrap();
    let mut files = serde_json::Map::new();
    for f in ["plugin.json", exe, "contracts/counter.contract.json"] {
        files.insert(
            f.into(),
            Value::String(format!(
                "{:x}",
                Sha256::digest(std::fs::read(root.join(f)).unwrap())
            )),
        );
    }
    let lock = json!({"lockVersion":1,"pluginId":"example.counter","pluginVersion":"0.1.0","artifact":{"profile":"native-executable","target":Target::current(),"abi":"native-executable","runtime":[],"availability":{"built":true,"distributed":false,"tested":[]}},"interfaces":{"example.counter":contract().identity()},"files":files});
    std::fs::write(
        root.join("plugin.lock.json"),
        serde_json::to_vec(&lock).unwrap(),
    )
    .unwrap();
    Package(root)
}
fn options(mode: &str) -> LoadOptions {
    LoadOptions {
        configuration: json!({"mode":mode}),
        ..Default::default()
    }
}
fn host() -> Host {
    Host::new(Limits {
        call_timeout_ms: 300,
        shutdown_timeout_ms: 150,
        startup_timeout_ms: 3000,
        max_log_bytes: 1024,
        ..Default::default()
    })
    .unwrap()
}
#[tokio::test]
async fn repeated_unload_reaps_and_instances_are_independent() {
    let pkg = package();
    let h = host();
    let a = h
        .load(pkg.0.join("plugin.json"), options("normal"))
        .await
        .unwrap();
    let b = h
        .load(pkg.0.join("plugin.json"), options("normal"))
        .await
        .unwrap();
    assert_ne!(a.instance_id(), b.instance_id());
    assert_eq!(
        a.invoke(&contract(), "add", json!({"amount":12}))
            .await
            .unwrap(),
        json!({"count":"12"})
    );
    assert_eq!(
        b.invoke(&contract(), "get", json!({})).await.unwrap(),
        json!({"count":"0"})
    );
    let pid = a.pid().unwrap();
    h.unload(&a.instance_id()).await.unwrap();
    h.unload(&a.instance_id()).await.unwrap();
    #[cfg(target_os = "linux")]
    assert!(!PathBuf::from(format!("/proc/{pid}")).exists());
    h.shutdown().await.unwrap();
}
#[tokio::test]
async fn deadline_does_not_release_serial_slot_or_undo_effects() {
    let pkg = package();
    let h = host();
    let p = h
        .load(pkg.0.join("plugin.json"), options("ignore-cancel"))
        .await
        .unwrap();
    let e = p
        .context()
        .with_timeout(Duration::from_millis(15))
        .invoke(&contract(), "add", json!({"amount":1}))
        .await
        .unwrap_err();
    assert_eq!(e.stable_code(), "DEADLINE_EXCEEDED");
    assert_eq!(
        p.invoke(&contract(), "get", json!({}))
            .await
            .unwrap_err()
            .stable_code(),
        "REENTRANT_CALL"
    );
    let value = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match p.invoke(&contract(), "get", json!({})).await {
                Ok(v) => break v,
                Err(e) if e.stable_code() == "REENTRANT_CALL" => {
                    tokio::time::sleep(Duration::from_millis(5)).await
                }
                Err(e) => panic!("{e}"),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(value, json!({"count":"1"}));
    h.shutdown().await.unwrap();
}
#[tokio::test]
async fn snapshot_failure_and_author_task_remain_draining() {
    for mode in ["snapshot-error", "uncooperative-task"] {
        let pkg = package();
        let h = host();
        let p = h
            .load(pkg.0.join("plugin.json"), options(mode))
            .await
            .unwrap();
        assert!(
            h.reload(&p.instance_id(), ReloadOptions::default())
                .await
                .is_err()
        );
        assert_eq!(p.status(), Status::Draining);
        assert_eq!(
            p.invoke(&contract(), "get", json!({}))
                .await
                .unwrap_err()
                .stable_code(),
            "NOT_READY"
        );
        let _ = h.shutdown().await;
        assert_eq!(p.status(), Status::Stopped);
    }
}
#[tokio::test]
async fn failed_initialize_cleanup_error_exit_and_noisy_pipes() {
    let pkg = package();
    let h = host();
    assert!(
        h.load(pkg.0.join("plugin.json"), options("initialize-error"))
            .await
            .is_err()
    );
    assert_eq!(h.list()[0].status, Status::Failed);
    let p = h
        .load(pkg.0.join("plugin.json"), options("shutdown-error"))
        .await
        .unwrap();
    assert!(h.unload(&p.instance_id()).await.is_err());
    assert_eq!(p.status(), Status::Stopped);
    let p = h
        .load(pkg.0.join("plugin.json"), options("logs"))
        .await
        .unwrap();
    assert_eq!(
        p.invoke(&contract(), "get", json!({})).await.unwrap(),
        json!({"count":"0"})
    );
    h.unload(&p.instance_id()).await.unwrap();
    let (logs, dropped) = p.logs();
    assert!(logs.iter().map(|x| x.text.len()).sum::<usize>() <= 1024);
    assert!(dropped > 0);
    let p = h
        .load(pkg.0.join("plugin.json"), options("exit-call"))
        .await
        .unwrap();
    assert!(
        p.invoke(&contract(), "add", json!({"amount":1}))
            .await
            .is_err()
    );
    let _ = h.shutdown().await;
}
#[tokio::test]
async fn tampered_inventory_rejected_before_execution() {
    let pkg = package();
    std::fs::write(pkg.0.join("contracts/counter.contract.json"), b"{}").unwrap();
    assert!(
        host()
            .load(pkg.0.join("plugin.json"), LoadOptions::default())
            .await
            .is_err()
    );
}
