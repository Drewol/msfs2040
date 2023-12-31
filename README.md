Small experiment connecting a Raspberry Pi Pico to Flight Simulator.

## Build

### Firmware

```shell
cargo build -p device --target thumbv6m-none-eabi
elf2uf2-rs target/thumbv6m-none-eabi/debug/device -d
```

### Desktop

`cargo run -p host`
