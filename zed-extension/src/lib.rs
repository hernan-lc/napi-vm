use std::fs;

use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, Extension, GithubReleaseOptions,
    LanguageServerId, LanguageServerInstallationStatus, Os, Worktree,
};

const GITHUB_REPO: &str = "nglmercer/napi-vm";
const BINARY_NAME: &str = "napi-vm-lsp";

struct NapiVmExtension {
    cached: Option<String>,
}

impl NapiVmExtension {
    fn binary_relpath(os: Os) -> String {
        match os {
            Os::Windows => format!("{BINARY_NAME}/{BINARY_NAME}.exe"),
            _ => format!("{BINARY_NAME}/{BINARY_NAME}"),
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

    fn local_binary(worktree: &Worktree) -> Option<String> {
        let root = worktree.root_path();
        let candidates = [
            format!("target/release/{BINARY_NAME}"),
            format!("target/debug/{BINARY_NAME}"),
            format!("target/release/{BINARY_NAME}.exe"),
            format!("target/debug/{BINARY_NAME}.exe"),
        ];
        for relative in candidates {
            if worktree.read_text_file(&relative).is_ok() {
                return Some(format!("{root}/{relative}"));
            }
        }
        None
    }

    fn ensure_language_server(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> zed::Result<String> {
        if let Some(path) = &self.cached
            && fs::metadata(path).is_ok()
        {
            return Ok(path.clone());
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
        let binary_path = format!("{version_dir}/{}", Self::binary_relpath(os));
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
        if let Some(local) = Self::local_binary(worktree) {
            return Ok(zed::Command::new(local));
        }
        let server = self.ensure_language_server(language_server_id)?;
        Ok(zed::Command::new(server))
    }
}

zed::register_extension!(NapiVmExtension);
