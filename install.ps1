# Install avm on Windows:
#   irm https://raw.githubusercontent.com/PrajaNova/avm/main/install.ps1 | iex
# Set $env:AVM_VERSION (e.g. 'v0.4.0') to pin a release.
$ErrorActionPreference = 'Stop'
$repo = 'prajanova/avm'
$version = if ($env:AVM_VERSION) { $env:AVM_VERSION } else { 'latest' }
$base = if ($env:AVM_DOWNLOAD_BASE) { $env:AVM_DOWNLOAD_BASE }  # a mirror, or a local folder (e2e)
    elseif ($version -eq 'latest') { "https://github.com/$repo/releases/latest/download" }
    else { "https://github.com/$repo/releases/download/$version" }
function Get-Asset($name, $out) {
    if ($base -match '^https?://') { Invoke-WebRequest "$base/$name" -OutFile $out -UseBasicParsing }
    else { Copy-Item (Join-Path $base $name) $out }
}
$asset = 'avm_windows_amd64.zip'
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("avm-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
try {
    Write-Host "Downloading $asset..."
    Get-Asset $asset "$tmp\$asset"

    # Verify against the release's checksums.txt before extracting.
    $sums = "$tmp\checksums.txt"
    try { Get-Asset 'checksums.txt' $sums } catch { $sums = $null }
    if ($sums) {
        $expected = Get-Content $sums | ForEach-Object {
            $parts = $_.Trim() -split '\s+'
            if ($parts.Count -ge 2 -and $parts[1].TrimStart('*') -eq $asset) { $parts[0].ToLower() }
        } | Select-Object -First 1
        $actual = (Get-FileHash "$tmp\$asset" -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) {
            throw "checksum mismatch for $asset`n  expected $expected`n  got      $actual`n  refusing to install. Report at https://github.com/$repo/issues"
        }
        Write-Host "Verified sha256"
    } elseif ($env:AVM_ALLOW_UNVERIFIED -eq '1') {
        Write-Warning "release has no checksums.txt; installing UNVERIFIED (AVM_ALLOW_UNVERIFIED=1)"
    } else {
        throw "release has no checksums.txt, so $asset can't be verified. Set `$env:AVM_ALLOW_UNVERIFIED='1' to install anyway."
    }

    $bin = Join-Path $env:LOCALAPPDATA 'avm\bin'
    New-Item -ItemType Directory -Force $bin | Out-Null
    Expand-Archive "$tmp\$asset" -DestinationPath $bin -Force

    $userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
    if (($userPath -split ';') -notcontains $bin) {
        [Environment]::SetEnvironmentVariable('PATH', "$bin;$userPath", 'User')
    }
    $line = 'Invoke-Expression ((& avm-bin shell-init pwsh) -join "`n")'
    if (-not (Test-Path $PROFILE) -or -not (Select-String -Path $PROFILE -SimpleMatch $line -Quiet)) {
        New-Item -ItemType Directory -Force (Split-Path $PROFILE) | Out-Null
        New-Item -ItemType File -Force $PROFILE | Out-Null
        Add-Content $PROFILE "`n# avm`n$line"
    }
    Write-Host "Installed avm-bin to $bin. Open a new PowerShell window to start using avm."
} finally {
    Remove-Item -Recurse -Force $tmp
}
