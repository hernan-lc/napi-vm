$ErrorActionPreference = 'Stop'
$sysroot = (& rustc --print sysroot).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Could not locate the Rust toolchain.' }
$hostLine = (& rustc -vV | Select-String '^host: ').ToString()
if ($LASTEXITCODE -ne 0) { throw 'Could not identify the Rust host.' }
$rustHost = $hostLine.Substring(6).Trim()
$bundledLld = Join-Path $sysroot "lib/rustlib/$rustHost/bin/rust-lld.exe"
if (-not (Test-Path $bundledLld)) { throw "Bundled LLVM linker missing: $bundledLld" }

# Alias selects the MSVC driver and avoids an LLVM/Chocolatey installation.
# MSVC above provides SDK libraries and cl.exe for C build scripts/fixtures.
$linkerDir = Join-Path $env:RUNNER_TEMP 'napi-vm-linkers'
New-Item -ItemType Directory -Force $linkerDir | Out-Null
Copy-Item $bundledLld (Join-Path $linkerDir 'lld-link.exe') -Force
Add-Content -Path $env:GITHUB_PATH -Value $linkerDir
foreach ($target in @('X86_64_PC_WINDOWS_MSVC', 'AARCH64_PC_WINDOWS_MSVC')) {
    Add-Content -Path $env:GITHUB_ENV -Value "CARGO_TARGET_${target}_LINKER=lld-link.exe"
}
& (Join-Path $linkerDir 'lld-link.exe') --version
if ($LASTEXITCODE -ne 0) { throw 'Could not execute lld-link.' }
