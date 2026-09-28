# Windows end-to-end suite (CI's windows job). Builds avm-bin from this
# checkout, installs it with this checkout's install.ps1 (checksum-verified,
# from a local folder), then drives real node/java/android installs through
# the .exe shims. Run on a disposable machine: it writes to $HOME and the
# user PATH.
$ErrorActionPreference = 'Stop'
$script:pass = 0; $script:fail = 0
function Check($desc, [scriptblock]$test) {
    try { $ok = & $test } catch { $ok = $false; $err = $_ }
    if ($ok) { $script:pass++; Write-Host "  ✓ $desc" -ForegroundColor Green }
    else { $script:fail++; Write-Host "  ✗ $desc $err" -ForegroundColor Red }
}
function Section($t) { Write-Host "`n== $t" -ForegroundColor Cyan }
function Out([scriptblock]$b) { (& $b 2>&1 | Out-String).Trim() }

Section 'install with install.ps1 (from this checkout)'
cargo build --release -p avm-cli --bin avm-bin
$dist = Join-Path $env:RUNNER_TEMP 'avm-dist'
New-Item -ItemType Directory -Force $dist | Out-Null
Compress-Archive -Force -Path target/release/avm-bin.exe -DestinationPath "$dist\avm_windows_amd64.zip"
$h = (Get-FileHash "$dist\avm_windows_amd64.zip" -Algorithm SHA256).Hash.ToLower()
"$h  avm_windows_amd64.zip" | Set-Content "$dist\checksums.txt"
$env:AVM_DOWNLOAD_BASE = $dist
& ./install.ps1
$env:PATH = "$env:LOCALAPPDATA\avm\bin;$env:PATH"
Check 'avm-bin.exe installed' { Test-Path "$env:LOCALAPPDATA\avm\bin\avm-bin.exe" }
Check 'profile hook written' { (Get-Content $PROFILE -Raw) -match 'shell-init pwsh' }
Invoke-Expression ((& avm-bin shell-init pwsh) -join "`n")
Check 'avm -v' { (Out { avm -v }) -match '^avm ' }
Check 'avm --version' { (Out { avm --version }) -match '^avm ' }

$W = Join-Path $HOME 'winproj'; $P = Join-Path $W 'project'
New-Item -ItemType Directory -Force $P | Out-Null
Set-Location $W

Section 'global and local alias with the same name, global and local env'
avm add -g hello 'echo hello-global' | Out-Null
Push-Location $P; avm init | Out-Null; avm add hello 'echo hello-local' | Out-Null; Pop-Location
Check 'outside: global alias' { (Out { avm hello }) -eq 'hello-global' }
Check 'project: local alias wins' { Push-Location $P; try { (Out { avm hello }) -eq 'hello-local' } finally { Pop-Location } }
avm env add -g AVM_E2E_SCOPE global | Out-Null
Push-Location $P; avm env add AVM_E2E_SCOPE local | Out-Null; Pop-Location
Check 'outside: global env' { (Out { avm-bin env --shell pwsh }) -match "AVM_E2E_SCOPE = 'global'" }
Check 'project: local env' { Push-Location $P; try { (Out { avm-bin env --shell pwsh }) -match "AVM_E2E_SCOPE = 'local'" } finally { Pop-Location } }

Section 'node: global 20, local 22, through node.exe shims'
avm plugin add node
avm node install 20 -g
Push-Location $P; avm node install 22; Pop-Location
Check 'node.exe shim (not .cmd)' { (Test-Path "$env:AVM_SHIM_DIR\node.exe") -and -not (Test-Path "$env:AVM_SHIM_DIR\node.cmd") }
# The runner has its own node/java, so check the managed binary actually ran.
$tools = Join-Path $HOME '.avm\tools'
Check 'outside: managed node 20' { (Out { node -p 'process.version + process.execPath' }) -match '^v20\..*\\.avm\\tools\\node\\' }
Check 'project: managed node 22' { Push-Location $P; try { (Out { node -p 'process.version + process.execPath' }) -match '^v22\..*\\.avm\\tools\\node\\' } finally { Pop-Location } }
Check 'npm.exe shim runs npm.cmd' { Push-Location $P; try { (Out { npm -v }) -match '^\d+\.' } finally { Pop-Location } }
$npmOut = Out { npm install -g cowsay }
Write-Host ($npmOut -split "`n" | Select-String 'avm:' | Out-String)
Check 'npm -g installed into the managed node 20' { @(Get-ChildItem "$tools\node\20.*\bin\cowsay.cmd" -ErrorAction SilentlyContinue).Count -gt 0 }
Check 'cowsay.exe shim created' { Test-Path "$env:AVM_SHIM_DIR\cowsay.exe" }
if (-not (Test-Path "$env:AVM_SHIM_DIR\cowsay.exe")) {
    Write-Host "  shims: $((Get-ChildItem $env:AVM_SHIM_DIR).Name -join ', ')"
    Write-Host "  node 20 bin: $((Get-ChildItem "$tools\node\20.*\bin").Name -join ', ')"
}
Check 'global npm package (installed under 20) runs in the project on 22' { Push-Location $P; try { (Out { cowsay moo }) -match 'moo' } finally { Pop-Location } }

Section 'java: global 17, local 21, JAVA_HOME'
avm plugin add java
avm java install 17 -g
Push-Location $P; avm java install 21; Pop-Location
Check 'outside: managed java 17' { ((Out { java -version }) -match 'version "17') -and ((Out { avm-bin env --shell pwsh }) -match 'JAVA_HOME = .*\\.avm\\tools\\java\\openjdk-17') }
Check 'project: managed java 21' { Push-Location $P; try { ((Out { java -version }) -match 'version "21') -and ((Out { avm-bin env --shell pwsh }) -match 'JAVA_HOME = .*openjdk-21') } finally { Pop-Location } }
Check 'java 21 JDK installed under ~/.avm' { @(Get-ChildItem "$tools\java\openjdk-21*\bin\java.exe" -ErrorAction SilentlyContinue).Count -gt 0 }

Section 'android: install, ANDROID_HOME, adb'
avm plugin add android
avm android install 35 -g
Check 'ANDROID_HOME points into ~/.avm' { (Out { avm-bin env --shell pwsh }) -match 'ANDROID_HOME = .*\\.avm\\tools\\android\\35' }
Check 'adb.exe shim runs the managed adb' { ((Out { adb version }) -match 'Android Debug Bridge') -and (Test-Path "$env:AVM_SHIM_DIR\adb.exe") }
Check 'platform android-35 installed' { Test-Path "$tools\android\35\sdk\platforms\android-35" }

# Last: the swapped-in "release" is hostname.exe, and self-update relinks shims.
Section 'self-update replaces the running avm-bin.exe (fake local release)'
$su = Join-Path $env:RUNNER_TEMP 'su'
New-Item -ItemType Directory -Force "$su\bin", "$su\pkg", "$su\api\repos\PrajaNova\avm\releases\tags" | Out-Null
Copy-Item "$env:LOCALAPPDATA\avm\bin\avm-bin.exe" "$su\bin\avm-bin.exe"
Copy-Item "$env:SystemRoot\System32\hostname.exe" "$su\pkg\avm-bin.exe"
Compress-Archive -Force "$su\pkg\avm-bin.exe" "$su\avm_windows_amd64.zip"
$zh = (Get-FileHash "$su\avm_windows_amd64.zip" -Algorithm SHA256).Hash.ToLower()
"$zh  avm_windows_amd64.zip" | Set-Content "$su\checksums.txt"
$u = ([uri]"$su").AbsoluteUri
@{ tag_name = 'v9.9.9'; assets = @(
    @{ name = 'avm_windows_amd64.zip'; browser_download_url = "$u/avm_windows_amd64.zip" },
    @{ name = 'checksums.txt'; browser_download_url = "$u/checksums.txt" }) } |
    ConvertTo-Json -Depth 4 | Set-Content "$su\api\repos\PrajaNova\avm\releases\tags\v9.9.9"
$env:AVM_GITHUB_API_URL = "$su\api"
$out = Out { & "$su\bin\avm-bin.exe" self-update --version 9.9.9 }
Remove-Item Env:AVM_GITHUB_API_URL
Check "self-update swaps the running exe ($out)" { $out -match '→ 9\.9\.9' }
Check 'new binary in place' { (Get-FileHash "$su\bin\avm-bin.exe").Hash -eq (Get-FileHash "$su\pkg\avm-bin.exe").Hash }
Check 'previous binary moved aside' { Test-Path "$su\bin\avm-bin.old.exe" }

Write-Host "`nwindows: $script:pass passed, $script:fail failed"
if ($script:fail -gt 0) { exit 1 }
