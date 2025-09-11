
Import-Module -Force .\tasks\lib\utils.psm1
$taskArgs = Get-TaskArgs -Data $taskData

$report = @()

$script = $taskArgs."script"

$scriptPath = ".\.tm_temp\$($taskId).ps1"
$stdoutPath = ".\.tm_temp\STDOUT-$taskId.ps1"
$stderrPath = ".\.tm_temp\STDERR-$taskId.ps1"

$script | Out-File -FilePath $scriptPath
Start-Sleep -Seconds 1

try {
    Start-Process -FilePath "powershell.exe" -ArgumentList "-File $scriptPath" -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -Wait -PassThru -NoNewWindow
}
catch {
    Write-Error "An error occurred: $($_.Exception.Message)"
    $report += "The following script failed with `"$($_.Exception.Message)`":`n$($taskArgs."script")"
}

$report += "STDOUT:`n$(Get-Content -Path $stdoutPath)"
$report += "STDERR:`n$(Get-Content -Path $stderrPath)"
$report += "The following script executed successfully:`n$($taskArgs."script")"
Sync-Report -Report $report -TaskID $taskId

Remove-Item -Path $scriptPath
Remove-Item -Path $stdoutPath
Remove-Item -Path $stderrPath