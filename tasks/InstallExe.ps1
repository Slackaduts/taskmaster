
Import-Module -Force .\tasks\lib\utils.psm1
$taskArgs = Get-TaskArgs -Data $taskData

$report = @()

$scriptPath = ".\.tm_temp\$($taskId).ps1"
$stdoutPath = ".\.tm_temp\STDOUT-$taskId.ps1"
$stderrPath = ".\.tm_temp\STDERR-$taskId.ps1"

$script | Out-File -FilePath $scriptPath
Start-Sleep -Seconds 1

try {
    Start-Process -FilePath $taskArgs."exe" -ArgumentList $taskArgs."args" -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -Wait -PassThru -NoNewWindow
}
catch {
    Write-Error "An error occurred: $($_.Exception.Message)"
    $report += "The following program failed with `"$($_.Exception.Message)`":`n$($taskArgs."exe")"
}

$report += "STDOUT:`n$(Get-Content -Path $stdoutPath)"
$report += "STDERR:`n$(Get-Content -Path $stderrPath)"
$report += "The following program executed successfully:`n$($taskArgs."exe")"
Sync-Report -Report $report -TaskID $taskId

Remove-Item -Path $scriptPath
Remove-Item -Path $stdoutPath
Remove-Item -Path $stderrPath