<div align="center">

<img src="logo.png" atl="et logo" width="120"/>

# ⚡ et 

**et is a free, open-source hardware programming language built for performance, readability, and simplicity.**

[![Version](https://img.shields.io/badge/Release-v1.0.0-blue.svg?style=flat-square)]()
[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-orange.svg?style=flat-square&logo=rust)]()
[![Target](https://img.shields.io/badge/Target-Arduino_C++-00979D.svg?style=flat-square&logo=arduino)]()
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)]()

[Quick Start](#-quick-start) • [How It Works](#️-how-it-works-under-the-hood) • [Syntax Reference](#-syntax-specification) • [Hardware Example](#️-full-application-blueprint-variable-motor-controller)

<br/>
</div>

---

**et** is a lightweight, whitespace-sensitive programming language designed to make hardware prototyping fast, intuitive, and beginner-friendly. 

Built entirely in Rust, the `et` compiler acts as a smart transpiler. It handles memory layout, state tracking, and conditional branching, translating your clean logic into beautifully formatted, production-ready Arduino C++. 

Instead of fighting with C++ boilerplate, missing semicolons, or confusing bracket structures, `et` allows you to focus purely on the hardware logic.

## 📦 Prerequisites

Before you can compile and flash `et` scripts, you need two standard tools installed on your machine:

1. **Rust & Cargo:** The `et` compiler is built in Rust. You need it to run the build engine.
   * [Install Rust here](https://www.rust-lang.org/tools/install) (Takes 2 minutes).
2. **Arduino IDE:** `et` generates standard `.ino` C++ files. You will use the standard Arduino tools to upload the final code to your hardware.
   * [Install Arduino IDE here](https://www.arduino.cc/en/software).

---

## 🛠️ Installation

1. Clone this repository to your local machine:
   ```bash
   git clone [https://github.com/YOUR_USERNAME/et_lang.git](https://github.com/YOUR_USERNAME/et_lang.git)
   cd et_lang
   ```
2. Create a new file called `main.et` and write your hardware logic.

---

## 🏃‍♂️ Workflow: How to Compile & Flash

Because `et` is a transpiler, getting your code onto a physical board is a two-step process. 

### Step 1: Compile the Code (Terminal)
Open your terminal in the project folder and run your script through the Rust compiler:
```bash
cargo run main.et
```
The compiler will read your `main.et` file, format the logic, and instantly generate a production-ready C++ file called `main.ino` in the same directory.

### Step 2: Flash the Board (Arduino IDE)
Now you need to send that generated C++ code to your physical hardware.
1. Open the **Arduino IDE**.
2. Click **File > Open** and select the `main.ino` file you just generated.
   * *Note: The Arduino IDE requires `.ino` files to be inside a folder with the exact same name. If a warning pops up asking to create a `main` folder and move the file, click **OK**.*
3. Plug your Arduino board into your computer via USB.
4. Go to **Tools > Board** and select your specific hardware (e.g., Arduino Uno).
5. Go to **Tools > Port** and select your USB connection.
6. Click the **Upload** button (the Right Arrow icon `→`) in the top left corner.

The Arduino software will beam the logic to your microchip, and your hardware will start running!

---

## ⚙️ How It Works (Under the Hood)

`et` is not just a text-replacer; it is a multi-stage, modular compiler pipeline written in Rust.

1. **The Lexer (`lexer.rs`):** Rips the raw `.et` text apart line-by-line. It strips comment markers, calculates whitespace indentation, and transforms plain text words into strongly-typed Rust data structures (`Token` enums).
2. **The Smart Scope Tracker:** Instead of `{ }` brackets, `et` uses Python-style indentation. The compiler maintains an `indent_stack` in memory, automatically detecting when a logical block opens and closes based purely on your tabs/spaces.
3. **The Transpiler Core (`generator.rs`):** Translates the abstract token array directly into clean C++. It automatically prepends `#include <Arduino.h>` to seamlessly bridge the gap between your custom syntax and standard hardware development tools.
4. **The Auto-Formatter:** The generator dynamically injects C++ indentation, ensuring the final `.ino` output looks like it was written by a senior embedded software engineer.

---

## 📖 Syntax Specification

### 1. Project Architecture
Every `et` program is split into two foundational execution blocks. 

```text
setup:
    # Code here runs exactly once at system startup

loop:
    # Code here loops indefinitely
```

### 2. State & Memory Management
Declare global data variables at the absolute top of your file using the `set` command. Under the hood, these are translated into strongly-typed C++ global integers.

```text
set led_pin 13
set wait_time 500
```

### 3. Digital Hardware Control (I/O)
Interact with physical hardware units using clear, readable English expressions:
* `pin [number/var] [out/in]` — Configures the pin orientation as an OUTPUT (`out`) or INPUT (`in`).
* `on [number/var]` — Drives a digital HIGH signal (3.3v/5v) to the target pin.
* `off [number/var]` — Drives a digital LOW signal (0v) to the target pin.
* `wait [number/var]` — Halts thread execution for a specific duration in milliseconds.

```text
setup:
    pin led_pin out

loop:
    on led_pin
    wait wait_time
```

### 4. Analog Hardware Control (PWM)
If you need variable power (like fading an LED or controlling drone motor speed), use the `power` command to send a PWM signal.
* `power [pin] [0-255]` — Sends an analog signal to the target pin.

```text
loop:
    power motor_pin 128
```

### 5. Data Streams (Input Reading)
To read a digital input safely, `et` uses the `into` keyword. This creates a highly readable data flow and automatically provisions a local C++ tracking variable.

```text
# Syntax: read [pin] into [target_variable]
read button_pin into button_state
```

### 6. Live Math Engine
Update variables dynamically during the loop to calculate speeds, distances, or state changes.
* `math [variable] = [expression]` — Translates directly into C++ arithmetic.

```text
loop:
    math speed = speed + 10
```

### 7. Conditional Branching (Decisions)
Evaluate physical states dynamically without bracket boilerplate. The compiler determines the scope of execution blocks strictly by reading your indentation.

```text
if button_state == 1:
    on led_pin
else:
    off led_pin
```

### 8. Loop Iteration (Repeat Blocks)
Execute isolated routines a defined number of times. The engine automatically handles C++ `for` loop construction and index naming (e.g., `i0`, `i1`) behind the scenes.

```text
repeat 5:
    on led_pin
    wait 100
    off led_pin
    wait 100
```

---

## 🛠️ Full Application Blueprint: Variable Motor Controller

Here is a complete, working production blueprint written in `et` that showcases the live math and PWM engine:

```text
# --- et Motor Speed Control ---

set motor_pin 9
set speed 0

setup:
    pin motor_pin out

loop:
    # Set the motor to the current speed variable
    power motor_pin speed
    wait 100

    # Add 10 to the speed using the math engine
    math speed = speed + 10

    # Reset speed if it hits max PWM capacity (255)
    if speed > 255:
        math speed = 0
```