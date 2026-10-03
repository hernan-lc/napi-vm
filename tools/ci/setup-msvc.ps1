$ErrorActionPreference = 'Stop'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if (-not (Test-Path $vswhere)) { throw 'Visual Studio Installer (vswhere) is missing.' }
$installation = (& $vswhere -latest -products '*' -prerelease -property installationPath).Trim()
if ($LASTEXITCODE -ne 0 -or -not $installation) { throw 'Visual Studio was not found.' }
$vcvars = Join-Path $installation 'VC/Auxiliary/Build/vcvarsall.bat'
if (-not (Test-Path $vcvars)) { throw "MSVC environment script missing: $vcvars" }
$targetArch = if ($env:RUNNER_ARCH -eq 'ARM64') { 'arm64' } else { 'x64' }

# Import the native compiler and SDK environment without a JavaScript action
# that still declares the deprecated Node 20 runtime.
$start = [System.Diagnostics.ProcessStartInfo]::new()
$start.FileName = $env:ComSpec
$start.Arguments = '/d /s /c ""' + $vcvars + '" ' + $targetArch + ' >nul && set"'
$start.UseShellExecute = $false
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$process = [System.Diagnostics.Process]::Start($start)
$stdout = $process.StandardOutput.ReadToEndAsync()
$stderr = $process.StandardError.ReadToEndAsync()
$process.WaitForExit()
$output = $stdout.GetAwaiter().GetResult()
$errors = $stderr.GetAwaiter().GetResult()
if ($process.ExitCode -ne 0) { throw "MSVC environment setup failed: $errors" }

$variables = @{}
foreach ($line in ($output -split "`r?`n")) {
    if ($line -match '^([A-Za-z_][A-Za-z0-9_()]*)=(.*)$') {
        $variables[$Matches[1]] = $Matches[2]
    }
}
if ($variables['VSCMD_ARG_TGT_ARCH'] -ne $targetArch -or
    -not $variables['INCLUDE'] -or -not $variables['LIB'] -or
    -not $variables['WindowsSdkDir']) {
    throw 'MSVC did not configure the requested architecture and Windows SDK.'
}
foreach ($name in $variables.Keys) {
    $value = $variables[$name]
    if ([Environment]::GetEnvironmentVariable($name) -ne $value) {
        Add-Content -Path $env:GITHUB_ENV -Value "$name=$value"
        [Environment]::SetEnvironmentVariable($name, $value)
    }
}
if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) { throw 'cl.exe is unavailable after setup.' }
Write-Host "MSVC $($variables['VCToolsVersion']), Windows SDK $($variables['WindowsSDKVersion']), target $targetArch"
