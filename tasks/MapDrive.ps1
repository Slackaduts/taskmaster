
Import-Module -Force .\tasks\lib\utils.psm1
$taskArgs = Get-TaskArgs -Data $taskData

$report = @()

foreach ($letter in $taskArgs."drives".Keys) {
    $path = $taskArgs."drives"[$letter]
    try {
        New-PSDrive -Name $letter.ToUpper() -PSProvider "FileSystem" -Root $path -Persist -Scope Global -ErrorAction Stop
    }
    catch {
        Write-Error "An error occurred: $($_.Exception.Message)"
        $report += "Attempted to map drive letter $letter to path $path, failed with `"$($_.Exception.Message)`""
        continue
    }

    $actionStr = "Mapped drive letter $letter to path $path."
    $report += $actionStr
}

Sync-Report -Report $report -TaskID $taskId