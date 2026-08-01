use std::path::Path;

use zed_extension_api::{self as zed, Extension, LanguageServerId, Worktree};

struct NapiVmExtension;

impl Extension for NapiVmExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<zed::Command> {
        let node = zed::node_binary_path()?;
        let root_path = worktree.root_path();
        let root = Path::new(&root_path);

        // Prefer a checkout-local server. The fallback is the copy shipped in
        // the npm package, which makes the extension work in a consumer app.
        let relative_server = if worktree.read_text_file("lsp/server.cjs").is_ok() {
            Path::new("lsp/server.cjs")
        } else {
            Path::new("node_modules/napi-vm/lsp/server.cjs")
        };
        let server = root.join(relative_server);

        Ok(zed::Command::new(node).args([
            server.to_string_lossy().into_owned(),
        ]))
    }
}

zed::register_extension!(NapiVmExtension);
