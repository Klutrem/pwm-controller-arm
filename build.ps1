#format rust file
rustfmt ./src/main.rs
# Run 'cargo build' to build the project
cargo build --release

# Define source and destination paths
$sourcePath = "target\thumbv7m-none-eabi\release\pwm-fan"
$destinationPath = "build\pwm-fan.elf"

# Copy the file from source to destination
if (Test-Path $sourcePath) {
  Copy-Item -Path $sourcePath -Destination $destinationPath -Force
  Write-Host "File copied successfully to $destinationPath"
}
else {
  Write-Host "Source file not found: $sourcePath"
}
