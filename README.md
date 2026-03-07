# led-dot-clk

Shelf clock with LED dot matrix display and NTP time synchronization on ESP32-C2 written in Rust (and embassy)

## Build

```bash
# Add target riscv32imc-unknown-none-elf
rustup target add riscv32imc-unknown-none-elf

# Build
cargo build --release
```
