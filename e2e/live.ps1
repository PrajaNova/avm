# The live journey on Windows (PowerShell 7): install the published avm with the published
# install.ps1, install a plugin from the live registry, then check global vs local aliases, env
# and node versions across folders. Mirrors live.sh. Run on a disposable machine (CI's
# windows-latest): it writes to $HOME and the user PATH.
$ErrorActionPreference = 'Stop'
$script:pass = 0; $script:fail = 0
function Section($t) { Write-Host "`n== $t" -ForegroundColor Cyan }
function Check($desc, [scriptblock]$test) {
    try { $ok = & $test } catch { $ok = $false; $err = $_ }
    if ($ok) { $script:pass++; Write-Host "  ✓ $desc" -ForegroundColor Green }
    else { $script:fail++; Write-Host "  ✗ $desc $err" -ForegroundColor Red }
}
function Out([scriptblock]$b) { (& $b 2>&1 | Out-String).Trim() }
function InDir($dir, [scriptblock]$b) { Push-Location $dir; try { Out $b } finally { Pop-Location } }

Section '1. install avm (published install.ps1 -> published release)'
# Default to the newest published build, prereleases included (stable `latest` can be an older CLI).
if (-not $env:AVM_VERSION) {
    $headers = @{}; if ($env:GITHUB_TOKEN) { $headers.Authorization = "Bearer $env:GITHUB_TOKEN" }
    $releases = Invoke-RestMethod 'https://api.github.com/repos/PrajaNova/avm/releases?per_page=30' -Headers $headers
    $env:AVM_VERSION = ($releases | Where-Object { $_.tag_name -like 'v*' } | Select-Object -First 1).tag_name
}
Write-Host "installing avm $env:AVM_VERSION"
Invoke-RestMethod https://raw.githubusercontent.com/PrajaNova/avm/main/install.ps1 | Invoke-Expression
$env:PATH = "$env:LOCALAPPDATA\avm\bin;$env:PATH"
Check 'avm-bin.exe installed' { Test-Path "$env:LOCALAPPDATA\avm\bin\avm-bin.exe" }
Invoke-Expression ((& avm-bin shell-init pwsh) -join "`n")
Check 'avm -v prints a version' { (Out { avm -v }) -match '^avm ' }
if ($env:AVM_VERSION -ne 'latest') { Check 'installed the requested release' { (Out { avm -v }) -match [regex]::Escape($env:AVM_VERSION.TrimStart('v')) } }

Section '2. install a plugin from the live registry'
# `plugin add|remove|available` (older CLI) or `plugin install|uninstall|search` (newer).
& avm-bin plugin install --help *> $null
$newCli = ($LASTEXITCODE -eq 0)
function PluginAdd($name) { if ($newCli) { avm plugin install $name } else { avm plugin add $name } }
function PluginFind { if ($newCli) { avm plugin search node } else { avm plugin available } }
Check 'the registry lists node' { (Out { PluginFind }) -match 'node' }
PluginAdd node | Out-Null
Check 'node plugin binary is in place' { Test-Path "$HOME\.avm\plugins\avm-plugin-node\bin\avm-plugin.exe" }
Check 'avm plugin list shows node' { (Out { avm plugin list }) -match 'node' }
Check 'an unknown plugin name is rejected and named' { (Out { PluginAdd no-such-plugin-xyz }) -match 'no-such-plugin-xyz' }

Section '3. global alias, usable from anywhere'
Set-Location $HOME
avm alias add -g hello 'echo hello-global' | Out-Null
Check 'run the global alias' { (Out { avm hello }) -eq 'hello-global' }
Check 'avm run does the same' { (Out { avm run hello }) -eq 'hello-global' }
Check 'which says it is a global alias' { (Out { avm which hello }) -match 'global alias' }
Check 'alias list shows it' { (Out { avm alias list }) -match 'hello' }
avm env add -g WHERE global | Out-Null
Check 'avm env exports the global value' { (Out { avm-bin env --shell pwsh }) -match "WHERE = 'global'" }

Section '4. two project folders with local aliases and env'
$A = Join-Path $HOME 'work\proj-a'; $B = Join-Path $HOME 'work\proj-b'; $Plain = Join-Path $HOME 'work\plain'
New-Item -ItemType Directory -Force "$A\src\deep", $B, $Plain | Out-Null
Push-Location $A; avm init | Out-Null; avm alias add hello 'echo hello-a' | Out-Null; avm alias add only-a 'echo only-in-a' | Out-Null; avm env add WHERE proj-a | Out-Null; Pop-Location
Push-Location $B; avm init | Out-Null; avm alias add hello 'echo hello-b' | Out-Null; avm alias add only-b 'echo only-in-b' | Out-Null; avm env add WHERE proj-b | Out-Null; Pop-Location
Check 'config files were created' { (Test-Path "$A\.avm.json") -and (Test-Path "$B\.avm.json") }

Section '5. the right alias wins in each folder'
Check 'proj-a: local alias wins' { (InDir $A { avm hello }) -eq 'hello-a' }
Check 'proj-b: local alias wins' { (InDir $B { avm hello }) -eq 'hello-b' }
Check 'plain folder: global alias' { (InDir $Plain { avm hello }) -eq 'hello-global' }
Check 'a subfolder inherits the project alias' { (InDir "$A\src\deep" { avm hello }) -eq 'hello-a' }
Check 'an alias only in proj-a runs there' { (InDir $A { avm only-a }) -eq 'only-in-a' }
Check "proj-a's alias does not leak into proj-b" { (InDir $B { avm only-a }) -match 'not found' }
Check 'which: local alias in proj-a' { (InDir $A { avm which hello }) -match 'local alias' }
Check 'which: global alias in a plain folder' { (InDir $Plain { avm which hello }) -match 'global alias' }
Check 'resolve prints the command' { (InDir $A { avm resolve hello }) -match 'echo hello-a' }

Section '6. env: local overrides global'
Check 'proj-a env wins' { (InDir $A { avm-bin env --shell pwsh }) -match "WHERE = 'proj-a'" }
Check 'proj-b env wins' { (InDir $B { avm-bin env --shell pwsh }) -match "WHERE = 'proj-b'" }
Check 'plain folder sees the global env' { (InDir $Plain { avm-bin env --shell pwsh }) -match "WHERE = 'global'" }

Section '7. node: global 20, local 22 in proj-a only'
avm node install 20 -g | Out-Null
Push-Location $A; avm node install 22 | Out-Null; Pop-Location
Check 'plain folder uses the global node' { (InDir $Plain { node --version }) -match '^v20\.' }
Check 'proj-a uses its own node' { (InDir $A { node --version }) -match '^v22\.' }
Check 'proj-b has no pin, so it uses the global node' { (InDir $B { node --version }) -match '^v20\.' }
Check 'a subfolder of proj-a inherits node 22' { (InDir "$A\src\deep" { node --version }) -match '^v22\.' }
Check 'which node in proj-a is the local pin' { (InDir $A { avm which node }) -match '22' }
Check 'avm node list shows the selection' { (InDir $A { avm node list }) -match 'Selected version: 22' }

Section '8. removing aliases falls back, then disappears'
Push-Location $A; avm alias remove hello | Out-Null; Pop-Location
Check 'proj-a now runs the global hello' { (InDir $A { avm hello }) -eq 'hello-global' }
avm alias remove -g hello | Out-Null
Check 'now it is gone everywhere' { (InDir $Plain { avm hello }) -match 'not found' }

Section '9. wrong input is reported'
Check 'unknown alias' { (Out { avm no-such-alias }) -match "alias 'no-such-alias' not found" }
Check 'a typo in a subcommand' { (Out { avm-bin plugin instal }) -match 'unrecognized subcommand' }
Check 'a bad env name' { (Out { avm env add 'BAD KEY' v }) -match 'invalid env key' }

Write-Host "`nlive: $script:pass passed, $script:fail failed"
if ($script:fail -gt 0) { exit 1 }
