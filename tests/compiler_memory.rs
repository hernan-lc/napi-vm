//! Dedicated test process: RSS is unaffected by parallel test binaries.
#[test]
fn repeated_synthesized_compilation_releases_owned_ast_data() {
    let source = "class Base { constructor(x){this.x=x;} } class Derived extends Base { [1+1]=3; field=4; static value=5; } class Explicit { field=9; constructor(){this.x=1;} } var a,b; [a,b=()=>42]=[1];";
    for _ in 0..500 {
        drop(napi_vm::Interpreter::compile(source).unwrap());
    }
    #[cfg(target_os = "linux")]
    let before = rss_kib();
    for _ in 0..20000 {
        drop(napi_vm::Interpreter::compile(source).unwrap());
    }
    #[cfg(target_os = "linux")]
    assert!(
        rss_kib() <= before + 4096,
        "compiler retained more than 4 MiB after warmup: before {before}, after {}",
        rss_kib()
    );
}
#[cfg(target_os = "linux")]
fn rss_kib() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmRSS:")
                .map(|value| value.split_whitespace().next().unwrap().parse().unwrap())
        })
        .unwrap()
}
