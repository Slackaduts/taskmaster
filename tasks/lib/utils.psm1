function NormalizeString {
    param (
        [string] $String
    )

    return $String.ToLower() -replace "[^a-zA-Z]", ""
}

function Convert-PSObjectToHashtable #https://stackoverflow.com/a/34383413
{
    param (
        [Parameter(ValueFromPipeline)] $InputObject,
        [bool] $ShouldNormalize = $true
    )

    process
    {
        if ($null -eq $InputObject) { return $null }

        if ($InputObject -is [System.Collections.IEnumerable] -and $InputObject -isnot [string])
        {
            $collection = @(
                foreach ($object in $InputObject) { Convert-PSObjectToHashtable $object }
            )

            Write-Output -NoEnumerate $collection
        }
        elseif ($InputObject -is [psobject])
        {
            $hash = @{}

            foreach ($property in $InputObject.PSObject.Properties)
            {
                $property_name = $property.Name #TODO: REMOVE THIS EDIT
                # if ($true -eq $ShouldNormalize) { $property_name = NormalizeString $property.Name }
                $hash[$property_name] = (Convert-PSObjectToHashtable $property.Value).PSObject.BaseObject
            }

            $hash
        }
        else
        {
            $InputObject
        }
    }
}


function Get-HashtableData {
    param (
        $Data,
        [array] $ValidKeys
    )

    Write-Host "Get-HashtableData"

    

    if ($Data -isnot [System.Collections.Hashtable] -and $Data -isnot [System.Collections.Specialized.OrderedDictionary])
    { #We shouldn't need indexable array support on userside. Shouldn't.
        return $Data
    }

    [hashtable]$Data = $Data

    $normValidKeys = Format-NormalizeArray -Strings $ValidKeys

    # Write-Host $normValidKeys

    $i = 0
    foreach ($key in $ValidKeys) {
        # Write-Host "index: $i"
        $normDataKey = NormalizeString -String $key
        # Write-Host $normDataKey
        if ($normDataKey -in $normValidKeys) {
            $dataKeys = [array]$Data.Keys
            
            Write-Host "TEST BEGIN"
            Write-Host "TEST FINISHED"
            return $Data.$($dataKeys[$i])
        }

        $i += 1
    }

    return $null;
}


function Format-NormalizeArray() {
    param (
        [string[]] $Strings
    )

    $output = @()

    foreach ($str in $Strings) {
        $output += NormalizeString $str 
    }

    return $output
}


function Get-ScriptPath {
    $driveLetter = Split-Path -Path $PSCommandPath -Qualifier #gets drive letter
    $contentPath = Split-Path -Path $PSCommandPath -NoQualifier #gets path part that isn't the drive letter section

    # Get the UNC path of the drive, null if its local
    $remotePath = (Get-WmiObject Win32_NetworkConnection | Where-Object { $_.LocalName -eq $driveLetter }).RemoteName

    if ($null -ne $remotePath) {
        return Join-Path $remotePath $contentPath
    }

    return $PSCommandPath
}

function Assert-IsElevated() {
    return ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] 'Administrator')
}

function Start-SelfElevated() {
    param (
        [string]$ExecutionPolicy = "Bypass" 
    )
    # Elevate to admin if needed/specified
    $isAdmin = Assert-IsElevated
    if (-not $isAdmin) {
        $scriptPath = Get-ScriptPath
        Start-Process powershell.exe "-NoProfile -ExecutionPolicy $ExecutionPolicy -File `"$scriptPath`"" -Verb RunAs
        exit
    }
}


function Get-TaskArgs() {
    param (
        $Data
    )

    return (ConvertFrom-Json -InputObject $Data | Convert-PSObjectToHashtable)
}


function Add-Timestamp {
    param (
        [string] $InputString,
        [bool] $IncludeUsername = $true
    )

    $timestamp = Get-Date -Format "MM-dd-yyyy HH:mm:ss"
    if ($IncludeUsername) {
        $InputString = "$($Env:UserName) | $InputString"
    }
    $fmt = "$timestamp | $InputString"
    return $fmt
}


function Sync-Report {
    param (
        [String[]] $Report,
        [string] $TaskID,
        [bool] $ShowResponse = $true
    )

    $body = ConvertTo-Json -InputObject $Report
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:3030/$TaskID" -Method Post -Body $body -ContentType "application/json"

    if ($ShowResponse) { Write-Host "Server response: $($response)" }
}