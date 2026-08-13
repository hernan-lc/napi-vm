use std::fs;

use zed_extension_api::{
    self as zed, settings::LspSettings, Architecture, DownloadedFileType, Extension,
    GithubReleaseOptions, LanguageServerId, LanguageServerInstallationStatus, Os, Worktree,
};

const GITHUB_REPO: &str = "nglmercer/napi-vm";
const BINARY_NAME: &str = "napi-vm-lsp";
/// Escape hatch for local development: point this at a built binary.
const PATH_ENV_VAR: &str = "NAPI_VM_LSP_PATH";

struct NapiVmExtension {
    cached: Option<String>,
}

impl NapiVmExtension {
    /// The executable's file name inside the extracted release archive.
    ///
    /// The archives are packaged with the binary at their root
    /// (`tar czf … napi-vm-lsp`), so after extraction into `<version_dir>` the
    /// binary sits directly at `<version_dir>/napi-vm-lsp[.exe]` — with no
    /// intermediate directory.
    fn binary_name(os: Os) -> &'static str {
        match os {
            Os::Windows => "napi-vm-lsp.exe",
            _ => BINARY_NAME,
        }
    }

    fn asset_name(os: Os, arch: Architecture) -> zed::Result<String> {
        let name = match (os, arch) {
            (Os::Mac, Architecture::Aarch64) => "napi-vm-lsp-darwin-arm64.tar.gz",
            (Os::Mac, Architecture::X8664) => "napi-vm-lsp-darwin-x64.tar.gz",
            (Os::Linux, Architecture::Aarch64) => "napi-vm-lsp-linux-arm64.tar.gz",
            (Os::Linux, Architecture::X8664) => "napi-vm-lsp-linux-x64.tar.gz",
            (Os::Windows, Architecture::Aarch64) => "napi-vm-lsp-windows-arm64.zip",
            (Os::Windows, Architecture::X8664) => "napi-vm-lsp-windows-x64.zip",
            _ => {
                return Err(format!(
                    "unsupported platform for {BINARY_NAME}: {os:?}/{arch:?}"
                ));
            }
        };
        Ok(name.into())
    }

    /// An explicitly configured binary: Zed's
    /// `lsp.napi-vm.binary.path` setting first, then the `NAPI_VM_LSP_PATH`
    /// environment variable.
    fn configured_binary(
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Option<String> {
        if let Some(path) = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.binary)
            .and_then(|binary| binary.path)
        {
            return Some(path);
        }
        worktree
            .shell_env()
            .into_iter()
            .find(|(key, _)| key == PATH_ENV_VAR)
            .map(|(_, value)| value)
            .filter(|value| !value.is_empty())
    }

    /// A `napi-vm-lsp` already on `$PATH`. This is the local development path:
    ///
    /// ```sh
    /// cargo build --release --no-default-features --bin napi-vm-lsp
    /// export PATH="$PWD/target/release:$PATH"
    /// ```
    ///
    /// The binary is deliberately never inspected with a text-reading API —
    /// it is an executable, not a text file.
    fn binary_on_path(worktree: &Worktree) -> Option<String> {
        worktree.which(BINARY_NAME)
    }

    fn download_language_server(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> zed::Result<String> {
        if let Some(path) = &self.cached {
            if fs::metadata(path).is_ok() {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let (os, arch) = zed::current_platform();
        let asset_name = Self::asset_name(os, arch)?;
        let release = zed::latest_github_release(
            GITHUB_REPO,
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("release {} has no {asset_name}", release.version))?;

        let version_dir = format!("{BINARY_NAME}-{}", release.version);
        let binary_path = format!("{version_dir}/{}", Self::binary_name(os));
        if fs::metadata(&binary_path).is_err() {
            zed::set_language_server_installation_status(
                language_server_id,
                &LanguageServerInstallationStatus::Downloading,
            );
            let archive_kind = if asset_name.ends_with(".zip") {
                DownloadedFileType::Zip
            } else {
                DownloadedFileType::GzipTar
            };
            zed::download_file(&asset.download_url, &version_dir, archive_kind)
                .map_err(|error| format!("failed to download {asset_name}: {error}"))?;
            zed::make_file_executable(&binary_path)
                .map_err(|error| format!("failed to mark {binary_path} executable: {error}"))?;

            if let Ok(entries) = fs::read_dir(".") {
                for entry in entries.flatten() {
                    if entry.file_name().to_string_lossy().starts_with(BINARY_NAME)
                        && entry.file_name() != *version_dir
                    {
                        let _ = fs::remove_dir_all(entry.path());
                    }
                }
            }
        }

        self.cached = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl Extension for NapiVmExtension {
    fn new() -> Self {
        Self { cached: None }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<zed::Command> {
        // Resolution order: explicitly configured path → `$PATH` → the
        // platform binary from the latest GitHub release. Node.js is never
        // involved, and nothing is resolved out of the project's
        // `node_modules`.
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree).ok();
        let command = match Self::configured_binary(language_server_id, worktree)
            .or_else(|| Self::binary_on_path(worktree))
        {
            Some(path) => path,
            None => self.download_language_server(language_server_id)?,
        };

        let binary_settings = settings.and_then(|settings| settings.binary);
        Ok(zed::Command {
            command,
            args: binary_settings
                .as_ref()
                .and_then(|binary| binary.arguments.clone())
                .unwrap_or_default(),
            env: binary_settings
                .and_then(|binary| binary.env)
                .map(|env| env.into_iter().collect())
                .unwrap_or_else(|| worktree.shell_env()),
        })
    }
}

zed::register_extension!(NapiVmExtension);
