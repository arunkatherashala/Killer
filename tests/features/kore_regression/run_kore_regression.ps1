$release_bin = "A:\My workspace\killer_A15\killer\SOURCE\src\v2-rust\killer\target\release\killer-native.exe"
$testfile = "A:\My workspace\killer_A15\killer\tests\features\kore_regression\kore_regression_suite.killer"
if (-Not (Test-Path $release_bin)) {
    Write-Error "Release binary not found at $release_bin. Build it with cargo --release or adjust path."
    exit 1
}
& $release_bin $testfile
exit $LASTEXITCODE
