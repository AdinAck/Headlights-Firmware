# embassy-template-stm32f0

## Changing for other chips

There are **2** locations where changes need to be made:

### `.cargo/config.toml`
`runner = 'probe-rs run --chip YOUR_CHIP_HERE'`

### `Cargo.toml`
`embassy-stm32 = { version = "0.1.0", features = ["nightly", "defmt", "memory-x", "YOUR_CHIP_HERE", "time-driver-any", "exti", "unstable-pac"] }`
