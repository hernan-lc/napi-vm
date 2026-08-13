fn main() {
    let code = napi_vm::lsp::run();
    std::process::exit(code);
}
