#include <Arduino.h>

int motor_pin = 9;
int speed = 0;

void setup() {
  pinMode(motor_pin, OUTPUT);
}

void loop() {
  analogWrite(motor_pin, speed);
  delay(100);
  speed = speed + 10;
  if (speed > 255) {
    speed = 0;
  }
}
