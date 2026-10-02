//! JSON CLI and JSONL development adapter. All plugin logs stay outside RPC/stdout.
use napi_vm_plugin_host::{Host, LoadOptions, ReloadOptions};
use napi_vm_plugin_protocol::{Contract, PluginResult, RpcError};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
use tokio::io::{AsyncBufReadExt, BufReader};
fn configuration() -> PluginResult<Contract> {
    Contract::from_value(serde_json::from_str(include_str!(
        "../../../../contracts/trusted-plugins/generated/app-configuration.contract.json"
    ))?)
}
fn host() -> PluginResult<Host> {
    let host = Host::default();
    host.register(configuration()?, "get", |_, _| {
        Box::pin(async { Ok(json!({"value":"Hola"})) })
    })?;
    Ok(host)
}
fn options(runtime: Option<&str>) -> LoadOptions {
    let mut o = LoadOptions::default();
    o.runtime.runtime = runtime
        .filter(|s| *s != "native" && *s != "executable")
        .map(str::to_string);
    if std::env::var_os("NAPI_VM_PLUGIN_DEVELOPMENT").is_some() {
        o.runtime.verify_integrity = false;
    }
    o
}
fn required<'a>(args: &'a [String], index: usize, label: &str) -> PluginResult<&'a str> {
    args.get(index).map(String::as_str).ok_or_else(||RpcError::new("INVALID_ARGUMENT",format!("missing {label}; usage: trusted-host-rust invoke MANIFEST INTERFACE METHOD JSON [node|bun|native], serve MANIFEST [runtime], or MANIFEST RUNTIME greeter|counter|migrate [REPLACEMENT]")))
}
#[tokio::main]
async fn main() {
    let result = run(std::env::args().skip(1).collect()).await;
    if let Err(e) = result {
        eprintln!(
            "{}",
            serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
        );
        std::process::exit(1);
    }
}
async fn run(args: Vec<String>) -> PluginResult<()> {
    let h = host()?;
    let result = run_host(&h, &args).await;
    let cleanup = h.shutdown().await;
    result?;
    cleanup
}
async fn run_host(host: &Host, args: &[String]) -> PluginResult<()> {
    match required(args, 0, "command/manifest")? {
        "invoke" => {
            let p = host
                .load(
                    required(args, 1, "manifest")?,
                    options(args.get(5).map(String::as_str)),
                )
                .await?;
            let c = p.contract(required(args, 2, "interface")?)?;
            let result = p
                .invoke(
                    &c,
                    required(args, 3, "method")?,
                    serde_json::from_str(required(args, 4, "JSON input")?)?,
                )
                .await?;
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        }
        "serve" => {
            let p = host
                .load(
                    required(args, 1, "manifest")?,
                    options(args.get(2).map(String::as_str)),
                )
                .await?;
            println!(
                "{}",
                json!({"ready":true,"instanceId":p.instance_id(),"sessionId":p.metadata().session_id})
            );
            let mut lines = BufReader::new(tokio::io::stdin()).lines();
            loop {
                let line =
                    tokio::select! {r=lines.next_line()=>r?,_=tokio::signal::ctrl_c()=>break};
                let Some(line) = line else {
                    break;
                };
                let outcome = async {
                    let v: Value = serde_json::from_str(&line)?;
                    match v["command"].as_str() {
                        Some("invoke") => {
                            let id = v["interface"].as_str().ok_or_else(|| {
                                RpcError::new("INVALID_ARGUMENT", "interface missing")
                            })?;
                            p.invoke(
                                &p.contract(id)?,
                                v["method"].as_str().unwrap_or(""),
                                v["input"].clone(),
                            )
                            .await
                        }
                        Some("reload") => {
                            host.reload(
                                &p.instance_id(),
                                ReloadOptions {
                                    replacement_manifest: v["manifest"].as_str().map(PathBuf::from),
                                    force_without_state: v["forceWithoutState"]
                                        .as_bool()
                                        .unwrap_or(false),
                                    runtime: v["runtime"].as_str().map(str::to_string),
                                },
                            )
                            .await?;
                            Ok(serde_json::to_value(p.metadata())?)
                        }
                        Some("shutdown") => Ok(json!({"stopped":true})),
                        _ => Err(RpcError::new("INVALID_ARGUMENT", "unknown JSONL command")),
                    }
                }
                .await;
                let stop = outcome.as_ref().is_ok_and(|r| r["stopped"] == true);
                match outcome {
                    Ok(v) => println!("{}", json!({"ok":true,"result":v})),
                    Err(e) => println!("{}", json!({"ok":false,"error":e})),
                }
                if stop {
                    break;
                }
            }
            Ok(())
        }
        manifest => {
            let runtime = args.get(1).map(String::as_str);
            let scenario = args.get(2).map(String::as_str).unwrap_or("greeter");
            let p = host.load(manifest, options(runtime)).await?;
            match scenario {
                "greeter" => {
                    let c = p.contract("example.greeter")?;
                    let result = p.invoke(&c, "greet", json!({"name":"Ana"})).await?;
                    if result != json!({"message":"Hola, Ana"}) {
                        return Err(RpcError::new(
                            "INVALID_RESULT",
                            "greeting did not match common scenario",
                        ));
                    }
                    let domain = p
                        .invoke(&c, "greet", json!({"name":"invalid"}))
                        .await
                        .unwrap_err();
                    if domain.stable_code() != "APPLICATION_ERROR"
                        || domain.data["domainCode"] != "INVALID_NAME"
                    {
                        return Err(RpcError::new(
                            "INVALID_RESULT",
                            "domain error did not match common scenario",
                        ));
                    }
                    println!(
                        "{}",
                        json!({"scenario":"greeter","greeting":result,"domainError":domain.data["domainCode"],"instanceId":p.instance_id(),"sessionId":p.metadata().session_id})
                    );
                }
                "benchmark" => {
                    let c = p.contract("example.counter")?;
                    for _ in 0..10 {
                        p.invoke(&c, "add", json!({"amount":1})).await?;
                    }
                    let total = std::time::Instant::now();
                    let mut samples = Vec::new();
                    for _ in 0..30 {
                        let start = std::time::Instant::now();
                        p.invoke(&c, "add", json!({"amount":1})).await?;
                        samples.push(start.elapsed().as_secs_f64() * 1000.0);
                    }
                    let total_ms = total.elapsed().as_secs_f64() * 1000.0;
                    samples.sort_by(f64::total_cmp);
                    let result = p.invoke(&c, "get", json!({})).await?;
                    if result != json!({"count":"40"}) {
                        return Err(RpcError::new(
                            "INVALID_RESULT",
                            "benchmark workload state mismatch",
                        ));
                    }
                    println!(
                        "{}",
                        json!({"scenario":"benchmark","runtime":p.metadata().selected_runtime,"runtimeVersion":p.metadata().runtime_version,"buildMode":if cfg!(debug_assertions){"debug"}else{"release"},"warmup":10,"samples":30,"p50Ms":samples[14],"p95Ms":samples[28],"p99Ms":samples[29],"totalMs":total_ms,"final":result})
                    );
                }
                "counter" | "migrate" => {
                    let c = p.contract("example.counter")?;
                    let mut events = p.subscribe(&c, "changed", 16)?;
                    let first = p.invoke(&c, "add", json!({"amount":12})).await?;
                    let first_event = tokio::time::timeout(Duration::from_secs(2), events.recv())
                        .await
                        .map_err(|_| {
                            RpcError::new("DEADLINE_EXCEEDED", "counter event did not arrive")
                        })??;
                    let old_session = p.metadata().session_id;
                    let mut sessions = vec![old_session.clone()];
                    let snapshot;
                    if scenario == "migrate" {
                        let replacement = required(args, 3, "replacement manifest")?;
                        host.reload(
                            &p.instance_id(),
                            ReloadOptions {
                                replacement_manifest: Some(replacement.into()),
                                runtime: if replacement.contains("-rust")
                                    || replacement.contains("native")
                                {
                                    None
                                } else {
                                    runtime.filter(|r| *r != "native").map(str::to_string)
                                },
                                ..Default::default()
                            },
                        )
                        .await?;
                        snapshot = p.retained_snapshot();
                        sessions.push(p.metadata().session_id.clone());
                        if p.metadata().session_id == old_session {
                            return Err(RpcError::new(
                                "INVALID_RESULT",
                                "reload did not rotate session",
                            ));
                        }
                    } else {
                        host.reload(&p.instance_id(), ReloadOptions::default())
                            .await?;
                        snapshot = p.retained_snapshot();
                        sessions.push(p.metadata().session_id.clone());
                    }
                    let restored = p.invoke(&c, "get", json!({})).await?;
                    let second = p.invoke(&c, "add", json!({"amount":3})).await?;
                    let next_event = tokio::time::timeout(Duration::from_secs(2), events.recv())
                        .await
                        .map_err(|_| {
                            RpcError::new(
                                "DEADLINE_EXCEEDED",
                                "stable event subscription did not follow reload",
                            )
                        })??;
                    if scenario == "migrate" {
                        host.reload(
                            &p.instance_id(),
                            ReloadOptions {
                                replacement_manifest: Some(manifest.into()),
                                runtime: runtime
                                    .filter(|r| *r != "native" && *r != "executable")
                                    .map(str::to_string),
                                ..Default::default()
                            },
                        )
                        .await?;
                        sessions.push(p.metadata().session_id.clone());
                    }
                    let third = p.invoke(&c, "add", json!({"amount":2})).await?;
                    if first != json!({"count":"12"})
                        || restored != first
                        || second != json!({"count":"15"})
                        || third != json!({"count":"17"})
                    {
                        return Err(RpcError::new(
                            "INVALID_RESULT",
                            "counter migration changed state",
                        ));
                    }
                    println!(
                        "{}",
                        json!({"scenario":scenario,"instanceId":p.instance_id(),"sessions":sessions,"snapshot":snapshot,"first":first,"restored":restored,"second":second,"final":third,"events":[first_event,next_event]})
                    );
                }
                _ => return Err(RpcError::new("INVALID_ARGUMENT", "unknown scenario")),
            }
            Ok(())
        }
    }
}
