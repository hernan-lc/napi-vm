use napi_vm_plugin_host::{Host, LoadOptions, ReloadOptions};
use napi_vm_plugin_protocol::Contract;
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
fn required(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap_or_else(||panic!("{name} is required when explicitly running cross-runtime conformance; build and stage fixtures first"))
}
fn contract(name: &str) -> Contract {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/trusted-plugins/generated")
        .join(format!("{name}.contract.json"));
    Contract::from_value(serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()).unwrap()
}
#[tokio::test]
#[ignore = "requires staged Rust, Node and Bun artifacts; missing inputs are failures"]
async fn rust_host_actual_process_matrix() {
    tokio::time::timeout(Duration::from_secs(120), async {
        let host = Host::default();
        host.register(contract("app-configuration"), "get", |_, _| {
            Box::pin(async { Ok(json!({"value":"Hola"})) })
        })
        .unwrap();
        for (runtime, path) in [
            (Some("node"), required("NAPI_VM_GREETER_JS_MANIFEST")),
            (Some("bun"), required("NAPI_VM_GREETER_JS_MANIFEST")),
            (None, required("NAPI_VM_GREETER_RUST_MANIFEST")),
        ] {
            let mut o = LoadOptions::default();
            o.runtime.runtime = runtime.map(str::to_string);
            let p = host.load(path, o).await.unwrap();
            assert_eq!(
                p.invoke(&contract("greeter"), "greet", json!({"name":"Ana"}))
                    .await
                    .unwrap(),
                json!({"message":"Hola, Ana"})
            );
            assert_eq!(
                p.invoke(&contract("greeter"), "greet", json!({"name":"invalid"}))
                    .await
                    .unwrap_err()
                    .stable_code(),
                "APPLICATION_ERROR"
            );
            host.unload(&p.instance_id()).await.unwrap();
        }
        let mut o = LoadOptions::default();
        o.runtime.runtime = Some("node".into());
        let p = host
            .load(required("NAPI_VM_COUNTER_JS_MANIFEST"), o)
            .await
            .unwrap();
        let c = contract("counter");
        assert_eq!(
            p.invoke(&c, "add", json!({"amount":12})).await.unwrap(),
            json!({"count":"12"})
        );
        host.reload(
            &p.instance_id(),
            ReloadOptions {
                replacement_manifest: Some(required("NAPI_VM_COUNTER_RUST_MANIFEST")),
                runtime: Some("native".into()),
                force_without_state: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            p.invoke(&c, "add", json!({"amount":3})).await.unwrap(),
            json!({"count":"15"})
        );
        host.reload(
            &p.instance_id(),
            ReloadOptions {
                replacement_manifest: Some(required("NAPI_VM_COUNTER_JS_MANIFEST")),
                runtime: Some("node".into()),
                force_without_state: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            p.invoke(&c, "get", json!({})).await.unwrap(),
            json!({"count":"15"})
        );
        assert_ne!(p.retained_snapshot(), Value::Null);
        host.shutdown().await.unwrap();
    })
    .await
    .expect("conformance process matrix deadline exceeded");
}
