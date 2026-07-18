# PyTreXT Windows Service Installer
# Run as Administrator: PowerShell -ExecutionPolicy Bypass -File install-service.ps1

$serviceName = "testapp"
$displayName = "PyTreXT - testapp"
$description = "PyTreXT Production Service for testapp"
$binaryPath = "C:\Program Files\Python314\python.exe"
$arguments = '"C:\PYTREX-master\main.py"'
$workDir = "C:\PYTREX-master"

# Create service
New-Service -Name $serviceName `
    -DisplayName $displayName `
    -Description $description `
    -BinaryPathName "$binaryPath $arguments" `
    -StartupType Automatic

# Configure recovery
sc.exe failure $serviceName reset=86400 actions=restart/5000/restart/10000/restart/30000

# Set environment
[Environment]::SetEnvironmentVariable("PYTREX_ENV", "production", "Machine")
[Environment]::SetEnvironmentVariable("PYTREX_PORT", "8080", "Machine")

# Start service
Start-Service $serviceName

Write-Host "✅ Service '$serviceName' installed and started!"
Write-Host "   Check status: Get-Service $serviceName"
