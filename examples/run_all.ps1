$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptPath

Get-ChildItem -Directory | ForEach-Object {
    $exampleName = $_.Name
    Write-Host "Running cargo for $exampleName"
    & cargo run --quiet -p $exampleName
}
