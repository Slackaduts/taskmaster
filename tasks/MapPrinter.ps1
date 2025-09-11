
Import-Module -Force .\tasks\lib\utils.psm1
$taskArgs = Get-TaskArgs -Data $taskData

$report = @()

foreach ($printer in $taskArgs."printers") {
    try {
        Add-Printer -ConnectionName "\\$printer" -ErrorAction Stop
    }
    catch {
        Write-Error "An error occurred: $($_.Exception.Message)"
        $report += (Add-Timestamp -InputString "Attempted to add printer $printer, failed with $($_.Exception.Message)")
        continue
    }

    $report += (Add-Timestamp -InputString "Added printer $printer.")
}

Sync-Report -Report $report -TaskID $taskId