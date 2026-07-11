param(
  [string]$InstallerPath = "",
  [string]$TokenStackExecutable = "",
  [string]$EvidenceDirectory = ""
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
if (-not $EvidenceDirectory) {
  $EvidenceDirectory = Join-Path $repo "artifacts\windows-packaged-smoke"
}
$runtimeDirectory = Join-Path $EvidenceDirectory "fake runtime with spaces"
New-Item -ItemType Directory -Force $runtimeDirectory | Out-Null

if ($InstallerPath) {
  $installer = (Resolve-Path $InstallerPath).Path
  $install = Start-Process -FilePath $installer -ArgumentList "/S" -Wait -PassThru
  if ($install.ExitCode -ne 0) { throw "NSIS install failed with code $($install.ExitCode)." }
}

$fakeRuntime = Join-Path $runtimeDirectory "fake codex happy.exe"
& rustc (Join-Path $repo "src-tauri\tests\support\fake_codex.rs") -o $fakeRuntime
if ($LASTEXITCODE -ne 0) { throw "Could not compile the native fake Codex runtime." }

if (-not $TokenStackExecutable) {
  $candidates = @()
  $uninstallRoots = @(
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*"
  )
  foreach ($root in $uninstallRoots) {
    Get-ItemProperty $root -ErrorAction SilentlyContinue |
      Where-Object { $_.DisplayName -eq "TokenStack" } |
      ForEach-Object {
        if ($_.InstallLocation) {
          $installLocation = ([string]$_.InstallLocation).Trim().Trim('"')
          if ($installLocation) { $candidates += (Join-Path $installLocation "TokenStack.exe") }
        }
        if ($_.DisplayIcon) { $candidates += ($_.DisplayIcon -replace ',\d+$', '').Trim('"') }
      }
  }
  $candidates += @(
    (Join-Path $env:LOCALAPPDATA "TokenStack\TokenStack.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\TokenStack\TokenStack.exe")
  )
  $TokenStackExecutable = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $TokenStackExecutable -or -not (Test-Path $TokenStackExecutable)) {
  throw "TokenStack packaged executable was not found. Pass -TokenStackExecutable."
}
$TokenStackExecutable = (Resolve-Path $TokenStackExecutable).Path
$separator = [IO.Path]::DirectorySeparatorChar
$buildRoot = [IO.Path]::GetFullPath((Join-Path $repo "src-tauri\target")).TrimEnd($separator) + $separator
$userInstallRoot = [IO.Path]::GetFullPath($env:LOCALAPPDATA).TrimEnd($separator) + $separator
if ($TokenStackExecutable.StartsWith($buildRoot, [StringComparison]::OrdinalIgnoreCase)) {
  throw "Packaged smoke refuses a build-tree executable: $TokenStackExecutable"
}
if (-not $TokenStackExecutable.StartsWith($userInstallRoot, [StringComparison]::OrdinalIgnoreCase)) {
  throw "Installed executable is outside the per-user install location: $TokenStackExecutable"
}

function Invoke-PackagedSmoke([string]$Mode, [string[]]$RuntimeArguments, [string]$ExpectedRuntime, [string]$ExpectedSource) {
  $appDataDirectory = Join-Path $EvidenceDirectory "$Mode app data"
  $diagnosticsDirectory = Join-Path $EvidenceDirectory "$Mode diagnostics"
  New-Item -ItemType Directory -Force $appDataDirectory, $diagnosticsDirectory | Out-Null
  $arguments = @("--tokenstack-packaged-smoke") + $RuntimeArguments + @(
    "--app-data-dir", $appDataDirectory,
    "--diagnostics-dir", $diagnosticsDirectory
  )
  $env:TOKENSTACK_ENABLE_PACKAGED_SMOKE = "1"
  try {
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $TokenStackExecutable
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $arguments) {
      $startInfo.ArgumentList.Add($argument)
    }
    $process = [Diagnostics.Process]::Start($startInfo)
    if (-not $process) { throw "$Mode packaged smoke could not start TokenStack." }
    $process.WaitForExit()
    $smokeExitCode = $process.ExitCode
    $smokeOutput = $process.StandardOutput.ReadToEnd().Trim()
    $smokeError = $process.StandardError.ReadToEnd().Trim()
    $process.Dispose()
  } finally {
    Remove-Item Env:TOKENSTACK_ENABLE_PACKAGED_SMOKE -ErrorAction SilentlyContinue
  }
  if ($smokeExitCode -ne 0) {
    throw "$Mode packaged smoke exited with code $smokeExitCode. stdout=[$smokeOutput] stderr=[$smokeError]"
  }

  $diagnostics = Get-ChildItem $diagnosticsDirectory -Filter "tokenstack-diagnostics-*.json" |
    Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
  if (-not $diagnostics) { throw "$Mode packaged smoke did not export diagnostics." }
  $rawDiagnostics = Get-Content -Raw $diagnostics.FullName
  $report = $rawDiagnostics | ConvertFrom-Json
  if ($report.schemaVersion -ne 2) { throw "Unexpected diagnostics schema version." }
  if ($report.redaction.status -ne "sanitized") { throw "Diagnostics are not marked sanitized." }
  if ($report.diagnostics.latestAccountRun.initializeStatus -ne "ok") { throw "Handshake evidence is missing." }
  if ($report.diagnostics.latestAccountRun.childTerminated -ne $true) { throw "Child cleanup evidence is missing." }
  if ($report.diagnostics.latestAccountRun.dailyBucketCount -ne 1) { throw "Usage method evidence is missing." }
  if ($report.diagnostics.latestAccountRun.resetCreditCount -ne 3) { throw "Rate-limit method evidence is missing." }
  if ($report.diagnostics.selectedRuntime.nativeExecutablePath -ne $ExpectedRuntime) { throw "$Mode selected the wrong runtime." }
  if ($report.diagnostics.selectedRuntime.source -ne $ExpectedSource) { throw "$Mode reported the wrong runtime source." }
  if ($report.diagnostics.latestAccountRun.launchMode -ne "listen_stdio_no_mcp") { throw "$Mode reported the wrong launch mode." }
  if ($rawDiagnostics -match '(?i)bearer\s+[a-z0-9._-]+|authorization:|access[_-]?token|refresh[_-]?token|session=') {
    throw "Diagnostics contain a secret-bearing pattern."
  }
  Write-Output "PACKAGED_SMOKE_OK mode=$Mode diagnostics=$($diagnostics.FullName)"
}

Invoke-PackagedSmoke -Mode "explicit" -RuntimeArguments @("--runtime", $fakeRuntime) -ExpectedRuntime $fakeRuntime -ExpectedSource "configured"

$automaticRuntime = Join-Path $runtimeDirectory "codex.exe"
Copy-Item -Force $fakeRuntime $automaticRuntime
$originalPath = $env:PATH
$originalOverride = $env:TOKENSTACK_CODEX_BIN
try {
  Remove-Item Env:TOKENSTACK_CODEX_BIN -ErrorAction SilentlyContinue
  $env:PATH = @(
    $runtimeDirectory,
    (Join-Path $env:SystemRoot "System32"),
    $env:SystemRoot,
    (Join-Path $env:SystemRoot "System32\Wbem"),
    (Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0")
  ) -join ";"
  Invoke-PackagedSmoke -Mode "automatic" -RuntimeArguments @() -ExpectedRuntime $automaticRuntime -ExpectedSource "path"
} finally {
  $env:PATH = $originalPath
  if ($null -ne $originalOverride) { $env:TOKENSTACK_CODEX_BIN = $originalOverride }
}
