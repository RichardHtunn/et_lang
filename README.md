# et

et is a beginner-friendly language for the Arduino Uno R3 (ATmega328P). Its compiler is written in Rust and emits AVR machine code as Intel HEX. It does not generate C++ or need the Arduino build system.

## Build and upload

Install Rust and Cargo. Compile an example with:

    cargo run -- examples/blink.et

This writes `examples/blink.hex`. To upload through the Uno USB bootloader, install `avrdude` and pass the serial port:

    cargo run -- examples/blink.et /dev/ttyACM0

The upload command uses ATmega328P, the Arduino bootloader protocol, and 115200 baud. Compilation needs only Rust. Hardware upload has not been tested here.

## Language

Programs use indentation to mark blocks. Comments start with `#`.

    set led 13
    set pause 500

    task blink(pin, duration):
        on pin
        wait duration
        off pin
        wait duration

    setup:
        pin led out

    loop:
        repeat 2:
            blink(led, pause)

`setup:` runs once. `loop:` repeats forever. `set name value` declares an unsigned 16-bit value (0 through 65535). Its initial value can be a decimal number or an earlier `set` name. The compiler stores values in SRAM when code changes them and keeps unchanged values in machine-code constants.

Supported statements:

- `pin number out`, `pin number in`, and `pin number pullup` configure Uno digital pins 0 through 13. `pullup` enables the internal pull-up resistor; connect a button to ground and test for `0` when pressed.
- `on number` and `off number` drive a digital pin high or low.
- `read number into name` captures a digital input as `0` or `1`. The name may be declared with `set` or introduced by `read`.
- `power number value` sets 8-bit hardware PWM on Uno pin 9 or 10 using Timer1. Values are 0 through 255; a value computed at runtime is capped at 255.
- `wait milliseconds` waits using a 16 MHz CPU loop. It accepts a literal, a `set` value, or a task parameter.
- `math name = value`, `math name = left + right`, and `math name = left - right` update a value with unsigned 16-bit wraparound.
- `if left == right:` with optional `else:` branches on current values. Supported operators are `==`, `!=`, `<`, `<=`, `>`, and `>=`.
- `repeat count:` repeats a block. Up to four repeats may be nested.
- `task name(parameters):` declares a reusable block; `name(arguments)` calls it. Tasks expand at compile time. Recursion is rejected.

Values can be decimal numbers, declared names, or task parameters. Pin numbers must be fixed at compile time. Arithmetic supports one value or one addition or subtraction per statement. Analog PWM on Uno pins 3, 5, 6, and 11, analog input, and other boards remain future work. An unsupported statement produces a source-line error.

Try `examples/button.et` for digital input and `examples/fade.et` for mutable values and PWM.

## Compiler

`src/lexer.rs` reads indentation-aware lines, `src/parser.rs` builds structured statements, and `src/avr.rs` emits ATmega328P instruction words and Intel HEX. `src/main.rs` handles source files and optional upload. Waits hold the CPU; Timer1 PWM continues in hardware during a wait.
