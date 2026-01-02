# GPU External Power Supply

The GPU External Power Supply is a project that aims to replace any computer graphics cards' power delivery system with a completely custom board. This repository contains all code, firmware, and schematics required to build the card as well as documentation on how it works.

The sister repository [geps-app](https://github.com/starboundstitch/geps-app) contains the code for the GUI application.

## Summary

The External GPU Power Card is a designed to seamlessly integrate with existing GPUs. It works by replacing the Vcore and Vmem rails with new power stages that have reduced noise, an increased voltage range, and an increased maximum current which should provide additional stability and performance benefits.

The power supply also allows full control of its voltage characteristics to provide unlimited overclocking potential. The device integrates with our companion application to provide real time monitoring and adjustment using a GUI interface, allowing more informed decisions in the overclocking process to extract as much performance as possible.

## Specs

* 12 Phases (10+2)
* Vcore
  * 0-2V
  * 500A Tested
  * 900A Theoretical
  * 10 Phases
* Vcore
  * 0-2.3V
  * 100A Tested
  * 180A Theoretical
  * 2 Phases

## Hardware

This section contains the KICAD project and all the associated files.

We make extensive use of hierarchy in this project and have the following schematics:
* Vcore_stage
  * Single MOSFET that connects to the VRM Controller Vcore Channel
* Vmem_stage
  * Single MOSFET that connects to the VRM Controller Vmem Channel
* VRM_Controller
  * VRM Controller with required pinouts and filtering
* 12V_Supply
  * Input filtering and connectors
* Low_Current_Supply
  * 3.3V & 5V buck converters and LED status
* Onboard_Control
  * Buttons and Display
* U_Controller
  * Microcontroller and required connectors (including USB-C)

## Firmware

This section contains all micro-controller code and is using the rust language with the [stmf4XX-hal](https://github.com/stm32-rs/stm32f4xx-hal) library.

> [!CAUTION]
> The stm32f401RE that is used does NOT have enough SRAM to run in debug mode and it will NOT flash.

The controller should automatically build and flash with:

```
cargo run --release
```

Additionally, in the future we will support flashing via USB DFU mode (instructions TBD).

## License

All the code and hardware files in this repository are licensed according to the GPLv3 in the [LICENSE](https://github.com/starboundstitch/gpu-external-power-supply/blob/master/LICENSE) file.

In addition, many of the symbols and footprints are provided by the manufacturers for use and are thus liable under the manufactures' licenses.


This project was created by [@Xunnin](https://github.com/Xunnin), [@starboundstitch](https://github.com/starboundstitch), and [@StefClef](https://github.com/StefClef).
