# et

et is an indentation-based language for Arduino projects. Write pin control, sensor reads, timing, conditions, tasks, and arithmetic in ET; sketch targets can also call installed Arduino libraries using ET statements. Without a `board` line, the Uno R3 backend emits AVR machine code as Intel HEX. With `board uno_r3`, `board uno_r4_minima`, or `board nano33blesense_rev2`, ET emits an Arduino sketch. Sketch targets also support signed arithmetic and vectors for small ML algorithms.

## Download and install

1. Install [Git](https://git-scm.com/downloads) and [Rust with Cargo](https://rust-lang.org/tools/install/). Open a new terminal after installing Rust so `cargo` is on your path.
2. Download ET and enter the project folder:

   ```sh
   git clone https://github.com/RichardHtunn/et_lang.git
   cd et_lang
   ```

3. Check that ET runs:

   ```sh
   cargo run -- examples/blink.et
   ```

   This compiles the basic Uno R3 example to `examples/blink.hex`. ET needs no Arduino board connected to generate output.

For repeated use, build the ET executable once and run it directly after each `.et` edit:

```sh
cargo build --release
./target/release/et_lang examples/blink.et
```

On Windows, the executable is `target\release\et_lang.exe`. Rebuild it only when ET's Rust compiler source changes; editing a `.et` program does not require rebuilding ET.

For `.et` file icons and highlighting in VS Code, install the
[ET Language extension](vscode-et/README.md). The extension helps you edit ET;
the compiler installation above is still needed to generate Arduino sketches.

## Quick start: Uno R4 Minima

Install [Arduino IDE](https://www.arduino.cc/en/software) and its **Arduino UNO R4 Boards** package through Boards Manager. To try a project written in ET, run:

```sh
cargo run -- examples/r4_heartbeat_lcd.et
```

ET writes `target/et/r4_heartbeat_lcd/r4_heartbeat_lcd.ino`. Open that sketch in Arduino IDE, select **Arduino UNO R4 Minima** and the board's port, then use **Verify** and **Upload**. The heartbeat example uses the `LiquidCrystal` library and a parallel HD44780-compatible LCD wired as described in the `.et` file. Install the library through Library Manager if the IDE reports it missing. For a first board check without a sensor or display, use `examples/r4_arduino.et` instead.

If you prefer a terminal, [install Arduino CLI](https://docs.arduino.cc/arduino-cli/installation) and run:

```sh
arduino-cli core update-index
arduino-cli core install arduino:renesas_uno
arduino-cli lib update-index
arduino-cli lib install LiquidCrystal
arduino-cli board list
cargo run -- examples/r4_heartbeat_lcd.et
arduino-cli compile --fqbn arduino:renesas_uno:minima --upload --port PORT target/et/r4_heartbeat_lcd
```

Replace `PORT` with the port shown by `arduino-cli board list` (for example `/dev/ttyACM0` or `COM3`). The last command compiles and uploads in one step. To check the sketch without uploading, omit `--upload --port PORT`. ET generates the sketch, and Arduino tooling compiles the board code and installed libraries.

## Everyday workflow

1. Copy an example to a new `.et` file and set its `board` line for the target board. `board uno_r4_minima` generates an Arduino sketch; no `board` line uses the native Uno R3 machine-code backend.
2. Edit the `.et` file, then run `cargo run -- path/to/project.et` or the release executable shown above. Compiler errors include source line numbers.
3. For sketch targets, check the generated `target/et/<name>/<name>.ino`, then verify and upload it with Arduino IDE or CLI. Keep edits in the `.et` file because ET replaces the generated sketch next time. Use distinct file names for different projects; sketches with the same file name share an output folder.
4. Install a device's Arduino library once, then use ET's `use`, `device`, and `call` statements. The [heartbeat LCD example](examples/r4_heartbeat_lcd.et) shows the pattern. Start with [blink](examples/blink.et), [button](examples/button.et), or [fade](examples/fade.et) when learning the core syntax.

For the native Uno R3 backend, install `avrdude` to upload its `.hex` through the USB bootloader:

```sh
cargo run -- examples/blink.et /dev/ttyACM0
```

Replace the port with your board's port. This path uses ATmega328P, the Arduino bootloader protocol, and 115200 baud. Hardware upload has not been tested here. For Nano 33 BLE Sense Rev2, ET generates a sketch; follow the board-specific instructions below.

## Arduino C++ interoperability

Declare `board uno_r4_minima` (or `board uno_r3` / `board nano33blesense_rev2`) to use Arduino sketch output. A top-level `cpp:` block holds includes, objects, and C++ functions. A `cpp:` block inside `setup:`, `loop:`, a task, or another ET block runs C++ there. The code is copied into the sketch; Arduino tooling checks and compiles it. See [the mixed R4 example](examples/r4_mixed.et).

    board uno_r4_minima
    cpp:
        #include <Wire.h>
    set led 13
    setup:
        pin led out
        cpp:
            Wire.begin();
    loop:
        on led
        wait 500

ET scalar values are signed 32-bit integers on sketch targets and are named `v_name` in generated C++, so a value read with `read analog 0 into sensor` is available to C++ as `v_sensor`. Task parameters are named `p_name`. C++ code can call Arduino APIs and use any library that supports the selected board. Library APIs, hardware wiring, and board support remain the responsibility of Arduino tooling.

For full Arduino C++ compatibility, use one top-level `arduino:` block after the board declaration. Put a complete sketch, including `setup()` and `loop()`, inside it. ET copies the block to the `.ino` file without translating its statements. This mode cannot be mixed with ET `setup:`, `loop:`, tasks, or declarations. See [the full C++ R4 example](examples/r4_arduino.et). This is the path for devices such as LCDs when the library API is already known; ET does not need a separate command for each module.

## Intermediate projects in ET

The [Uno R4 heartbeat and LCD example](examples/r4_heartbeat_lcd.et) keeps its program logic in ET. It reads an analog sensor, counts threshold crossings, uses a 250 ms minimum gap between beats, calculates BPM over a 10-second window, and updates a parallel 16×2 LCD. Adjust the threshold and wiring for the actual hardware.

Sketch targets support these general ET features:

- `time into name` records `millis()` in an ET value.
- `elapsed start into name` calculates milliseconds since a recorded time, including timer wraparound for intervals up to about 24 days.
- `while left < right:` repeats a block while a comparison holds.
- `use "Library.h"` includes an installed Arduino library.
- `device name = Type(arguments)` creates a library object using literal constructor arguments or Arduino constants.
- `call object.method(arguments)` or `call function(arguments)` calls a library method or Arduino function. `call function(arguments) into name` stores an integer result in an ET value. Arguments can be ET values or parameters, numbers, quoted text, booleans, and uppercase Arduino constants such as `A0`.

For example, the LCD code is written in ET syntax:

    board uno_r4_minima
    use "LiquidCrystal.h"
    device lcd = LiquidCrystal(12, 11, 5, 4, 3, 2)
    set bpm 72
    setup:
        call lcd.begin(16, 2)
    loop:
        call lcd.print(bpm)
        wait 1000

`use` and `device` provide a general library bridge. Each LCD or sensor still needs its correct Arduino library, constructor arguments, and wiring. ET translates the calls and ET values; the Arduino compiler checks the library's methods and types. Complex C++ expressions and non-integer return values still require `cpp:` or `arduino:`.

## Language

Programs use indentation to mark blocks. ET comments start with `#`; C++ blocks retain C++ comments and preprocessor lines. The limits below describe the native Uno R3 backend when no `board` is declared.

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

Values can be decimal numbers, declared names, or task parameters. Pin numbers must be fixed at compile time. Arithmetic supports one value or one addition or subtraction per statement. Analog input is available on Uno channels 0 through 5 through `read analog`; PWM output on pins 3, 5, 6, and 11 is not supported by this native backend. An unsupported statement produces a source-line error.

Try `examples/button.et` for digital input and `examples/fade.et` for mutable values and PWM.

## First ML feature: learn from analog examples

ET can now train a small binary classifier as part of compilation. The CSV has
`value,label` columns: an Uno analog reading from 0 to 1023, and the desired
label `0` or `1`. At least two examples of each label are required. ET finds a
threshold and direction that classify the most training examples correctly,
then compiles that rule into AVR machine code. Training runs on your computer;
the Uno only makes predictions. The reported score is **training accuracy**, not
an estimate of performance on new readings.

Try the [knob example](examples/knob.et):

    cargo run -- examples/knob.et

Connect a potentiometer's outer legs to 5V and GND and its wiper to A0. The
built-in LED on pin 13 lights when ET predicts label `1`. The supplied
[`knob.csv`](examples/knob.csv) contains illustrative readings; replace them
with readings from your own circuit before relying on its predictions. ET
resolves the CSV path relative to the `.et` file.

The new statements are `learn name from "file.csv"` at the top level,
`read analog channel into name` for Uno channels 0 through 5, and
`predict model from value into name` inside a block. The prediction is `0` or
`1`, so it works with existing `if` statements. This Uno classifier handles
one analog input and two labels. Sensor data capture and held-out evaluation
are not implemented.

## Low-level ML in ET on Nano 33 BLE Sense Rev2

The [online tilt classifier](examples/online_tilt.et) implements a two-input
perceptron in ET. ET provides fixed-size vectors, indexed reads and writes,
dot products, signed arithmetic, tasks, and access to the board's IMU. The
program calculates predictions and updates its weights on the board when a
label button is pressed. No model or training CSV is generated on the computer
for this example.

    cargo run -- examples/online_tilt.et

This writes `target/et/online_tilt/online_tilt.ino`. Open that sketch with
Arduino IDE, select **Nano 33 BLE Sense Rev2**, install the **Arduino Mbed OS
Nano Boards** core and the
[Arduino_BMI270_BMM150](https://github.com/arduino-libraries/Arduino_BMI270_BMM150)
library, then upload. Connect a button from D2 to GND for label `0` and one
from D3 to GND for label `1`. Hold two different tilt poses and press the
corresponding buttons to train. The built-in LED displays the predicted label:
off for `0`, on for `1`. The model starts with zero weights each time the board
restarts; persistence is not implemented yet.

Nano ET programs start with `board nano33blesense_rev2`. They can declare
`vector weights 2`, use `put weights 0 value`, `get weights 0 into value`, and
`dot weights features into score`. `read imu into ax ay az` returns acceleration
in milligravity units as signed integers. Arduino sketch `set` values and arithmetic are
signed 32-bit integers. `math` supports a value or two operands with `+`, `-`,
`*`, or `/`. Addition, subtraction, multiplication, and dot products saturate
at the 32-bit limits. Division by zero returns `0`. An out-of-bounds vector
read returns `0`; an out-of-bounds write has no effect. These rules keep
generated code within fixed memory and avoid dynamic allocation. ET limits
declared sketch vectors to 32 KB total.

The Uno native backend and its compile-time `learn`/`predict` statements
remain available. On Nano and Uno R4, an ML algorithm can be written in ET with
math and vectors and executed on the board. `read imu` is specific to Nano 33
BLE Sense Rev2. The generated Nano sketch has been syntax-checked with host API
stubs and simulated, but has not yet been compiled with the actual Arduino
board core or tested on hardware.

## VS Code highlighting

The local [VS Code extension](vscode-et/README.md) adds syntax highlighting and
semantic colors for task names, task parameters, and mutable or read-only values.
It also recognizes `.et` files, gives them an alien-head icon, and configures
comments, brackets, and indentation.

## Compiler

`src/lexer.rs` reads indentation-aware lines and `src/parser.rs` builds structured statements. `src/avr.rs` emits ATmega328P instruction words and Intel HEX; `src/nano.rs` emits Arduino sketches for Uno R3, Uno R4 Minima, and Nano 33 BLE Sense Rev2. `src/main.rs` handles source files and optional native Uno upload. Native Uno waits hold the CPU; Timer1 PWM continues in hardware during a wait.
