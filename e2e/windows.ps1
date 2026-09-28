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
Check 'outside: node 20' { (Out { node -v }) -match '^v20\.' }
Check 'project: node 22' { Push-Location $P; try { (Out { node -v }) -match '^v22\.' } finally { Pop-Location } }
Check 'npm.exe shim runs npm.cmd' { Push-Location $P; try { (Out { npm -v }) -match '^\d+\.' } finally { Pop-Location } }
npm install -g cowsay 2>&1 | Out-Null
Check 'global npm package (installed under 20) runs in the project on 22' { Push-Location $P; try { (Out { cowsay moo }) -match 'moo' } finally { Pop-Location } }

Section 'java: global 17, local 21, JAVA_HOME'
avm plugin add java
avm java install 17 -g
Push-Location $P; avm java install 21; Pop-Location
Check 'outside: java 17' { (Out { java -version }) -match 'version "17' }
Check 'project: java 21' { Push-Location $P; try { (Out { java -version }) -match 'version "21' } finally { Pop-Location } }
Check 'JAVA_HOME in project → a 21 JDK' { Push-Location $P; try { (Out { avm-bin env --shell pwsh }) -match 'JAVA_HOME = .*openjdk-21' } finally { Pop-Location } }

Section 'android: install, ANDROID_HOME, adb'
avm plugin add android
avm android install 35 -g
Check 'ANDROID_HOME set' { (Out { avm-bin env --shell pwsh }) -match 'ANDROID_HOME = ' }
Check 'adb works' { (Out { adb version }) -match 'Android Debug Bridge' }

Write-Host "`nwindows: $script:pass passed, $script:fail failed"
if ($script:fail -gt 0) { exit 1 }
