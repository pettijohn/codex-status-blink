# codex-status-blink

`codex-status-blink` shows Codex usage remaining on a blink(1) USB LED. It runs `codex-status-json` and reads its JSON output.

The color changes with the remaining percentage:

- 100% is green.
- 50% is yellow.
- 0% is red.

## Requirements

- `codex-status-json` in `PATH`.
- A blink(1) USB LED with permission for the current user.

Install the udev rule on the host if the program cannot open the LED:

```sh
sudo tee /etc/udev/rules.d/51-blink1.rules <<'EOF'
ATTRS{idVendor}=="27b8", ATTRS{idProduct}=="01ed", \
  MODE:="666", GROUP="plugdev"
EOF
sudo udevadm control --reload
sudo udevadm trigger
```

Then reconnect the blink(1).

## Build

```sh
cargo build --release
```

## Run

This command shows the five-hour limit. This is the default limit.

```sh
cargo run --release
```

This command shows the weekly limit:

```sh
cargo run --release -- --limit weekly
```

This command shows both limits:

```sh
cargo run --release -- --limit both
```

With `--limit both`, LED 1 shows the five-hour limit. LED 2 shows the weekly limit. This option needs a blink(1) mk2 or later.

Use `--blink-serial` or `--blink-index` to select a blink(1):

```sh
cargo run --release -- --blink-serial 20001234
```

The initial poll interval is 30 seconds. If the selected limit does not change, the program waits 60 seconds, then 2 minutes, then 5 minutes. It continues at 5-minute intervals. A changed selected limit resets the interval to 30 seconds.

Use `--interval-secs` to change the initial poll interval. Use `--fade-ms 0` to set the LED color immediately. Use `--verbose` to show the usage and device status.

The program stops stored blink(1) patterns at startup. It turns the LED off during a normal stop.

## Test

```sh
cargo test
```

The tests do not need `codex-status-json` or a blink(1) device.
