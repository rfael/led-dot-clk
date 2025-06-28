# led-dot-clk

Shelf clock with LED dot matrix display and NTP time synchronization on ESP32-C2

## Build

```bash
git submodule update --init --recursive

# Apply patch to esp-hal
cd esp-hal
git apply ../esp-hal.patch
```
