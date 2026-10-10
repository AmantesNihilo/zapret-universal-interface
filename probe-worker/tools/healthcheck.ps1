param([Parameter(Mandatory = $true)][string]$BaseUrl)

$ErrorActionPreference = "Stop"
$base = $BaseUrl.TrimEnd("/")
$health = Invoke-RestMethod -Uri "$base/health" -Method Get -TimeoutSec 10
if (-not $health.ok -or $health.service -ne "zui-network-probe") {
    throw "Unexpected health response"
}
$json = Invoke-RestMethod -Uri "$base/json" -Method Get -TimeoutSec 10
if (-not $json.ok -or $json.marker -ne "zui-probe-v1") {
    throw "Unexpected JSON probe response"
}
Add-Type -AssemblyName System.Net.Http
$client = [System.Net.Http.HttpClient]::new()
$client.Timeout = [TimeSpan]::FromSeconds(10)
$request = [System.Net.Http.HttpRequestMessage]::new(
    [System.Net.Http.HttpMethod]::Get,
    "$base/bytes"
)
$request.Headers.Range = [System.Net.Http.Headers.RangeHeaderValue]::new(0, 31)
try {
    $range = $client.SendAsync($request).GetAwaiter().GetResult()
    $payload = $range.Content.ReadAsByteArrayAsync().GetAwaiter().GetResult()
    if ([int]$range.StatusCode -ne 206 -or $payload.Length -ne 32) {
        throw "Range probe failed"
    }
} finally {
    $request.Dispose()
    if ($null -ne $range) { $range.Dispose() }
    $client.Dispose()
}
Write-Output "ZUI probe endpoint is healthy: $base"
