# Headlights-Firmware
 
This repo contains firmware for:
- the headlights (stm32f031k6)
- the BLE relay (nrf52840)

as well as bindings for the [Swift app](https://github.com/AdinAck/Headlights-App).

# Embedded Rust

This firmware is completely written in Embedded Rust, fully utilizing the magical powers of the Rust compiler.

For concurrency and peripheral access, [embassy](https://github.com/embassy-rs/embassy) is used.

# Safety

This firmware is designed with safety as the number one priority. All detectable events will trigger a safe shutdown, and all errors are appropriately handled.

# Commands

These two devices (headlight and relay) exchange commands with a robust and adaptable command pattern.

The structures defining this behavior are shared between both binaries, so it is not possible to accidentally introduce a discrepency.

A CRC is used to validate commands, and commands are dispatched statically so no global allocator is needed.

---
[Hardware](https://github.com/AdinAck/Headlights-Hardware) | [App](https://github.com/AdinAck/Headlights-App)
