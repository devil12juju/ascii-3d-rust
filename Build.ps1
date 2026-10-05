param([string]$MingwBin)
$ErrorActionPreference = 'Stop'
$previousPath = $env:PATH
Push-Location $PSScriptRoot
try {
    if ($MingwBin) {
        if (-not (Test-Path -LiteralPath (Join-Path $MingwBin 'gcc.exe'))) {
            throw 'GNU compiler not found. Set -MingwBin to the folder containing gcc.exe.'
        }
        $env:PATH = $MingwBin + ';' + $env:PATH
    }
    $gcc = Get-Command gcc -ErrorAction SilentlyContinue
    $gnu = @(rustup toolchain list) | Where-Object { $_ -match '^stable-x86_64-pc-windows-gnu' }
    if ($gcc -and $gnu) { cargo +stable-x86_64-pc-windows-gnu build --release --locked }
    else { cargo build --release --locked }
    if ($LASTEXITCODE -ne 0) { throw 'Build failed. Configure a Rust toolchain with a working linker.' }
    Write-Host 'Build complete: target\release\ascii-3d-rust.exe (Windows) or target/release/ascii-3d-rust (Unix).'
} finally {
    $env:PATH = $previousPath
    Pop-Location
}
