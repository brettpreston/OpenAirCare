# OpenAirCare (Rust + Makepad, Linux)

Linux Rust program for AirPod hearing health functionality!

A small standalone app that configures the **Hearing Aid** features of
AirPods Pro 2 / AirPods Pro 3 from a Linux computer (x86_64 or ARM):

- enable / disable Hearing Aid (plus the listening-mode selector, because the
  feature only works in Transparency mode),
- enter your **audiogram** (8 bands per ear, dB HL) or load it from JSON,
- adjust amplification, balance, tone, ambient noise reduction and
  conversation boost
- "swipe to control amplification" toggle, reset.
- amplification above 1.00 (Apple's own maximum) is held at 1.00 until you
  confirm a warning: it is outside Apple's limits and could damage your
  hearing and/or the AirPods.
  
<img width="600" height="820" alt="OAC1" src="https://github.com/user-attachments/assets/14560964-d3e1-40a0-b005-5d2ff4b49637" />
<img width="600" height="820" alt="OAC2" src="https://github.com/user-attachments/assets/abf95a10-940b-4475-95f8-723de4a19429" />
<img width="600" height="820" alt="OAC3" src="https://github.com/user-attachments/assets/8a376072-6454-43a0-a595-3a28d24724b8" />
<img width="600" height="820" alt="OAC4" src="https://github.com/user-attachments/assets/ca2d5973-111f-472f-80e5-23b01a19b936" />

## Layout

```
openaircare/
  crates/airpods-proto   pure protocol (AACP framing, control commands, ATT PDUs,
                         104-byte hearing-aid codec) - no I/O, unit tested
  crates/airpods-link    BlueZ L2CAP transport (Linux) + session state machine,
                         plus a `mock` fake-AirPods for development on any OS
  app/                   the Makepad UI
```

## Prerequisites (Linux)

1. **BlueZ** with `bluetoothd` running (any modern distro / Raspberry Pi OS).
2. Your AirPods paired and **connected** as an audio device the normal way
   (`bluetoothctl connect XX:XX:...` or the desktop Bluetooth settings).
3. **Vendor-ID spoofing.** AirPods only expose the hearing-aid channel to
   hosts that identify as Apple. Add to `/etc/bluetooth/main.conf` under
   `[General]`:

   ```ini
   DeviceID = bluetooth:004C:0000:0000
   ```

   then `sudo systemctl restart bluetooth` and **re-pair** the AirPods (they
   cache the host's Device ID). Without this the app still connects (battery,
   listening mode, the Hearing Aid on/off switch) but the ATT channel on
   PSM 31 is refused (`Connection refused`), so audiogram/adjustments are
   unavailable and the Status page tells you so.
4. Only one AACP client per host: quit other programs that manage the airpods when using this one.
5. Build dependencies for Makepad (Ubuntu/Debian/Raspberry Pi OS):

   ```bash
   sudo apt-get install -y build-essential pkg-config clang libssl-dev \
     libx11-dev libxcursor-dev libxkbcommon-dev libxrandr-dev libxi-dev libxinerama-dev \
     libasound2-dev libpulse-dev libwayland-dev wayland-protocols \
     libegl1-mesa-dev libgl1-mesa-dev libgles2-mesa-dev libglx-dev libdrm-dev libgbm-dev \
     mesa-vulkan-drivers libdbus-1-dev
   ```

   (`libdbus-1-dev` is for the BlueZ D-Bus client used to find the AirPods.)

## Build & run

Rust stable (see `rust-toolchain.toml`). Makepad is pulled from git at a
pinned revision, so the first build takes a while.

```bash
cd openaircare
cargo test --workspace --features airpods-link/mock   # protocol + session tests
cargo run -p openaircare --release          # real BlueZ backend
cargo run -p openaircare --features mock    # fake AirPods, no hardware
```

Environment knobs:

| variable | effect |
|---|---|
| `RUST_LOG=debug` | log every AACP/ATT packet (hex) to stderr |
| `MAKEPAD_GPU=gl` | force the OpenGL ES backend if Vulkan misbehaves |
| `MAKEPAD=linux_direct` (build-time) | render straight to DRM/KMS without X11/Wayland (kiosk Pi) |
| `OPENAIRCARE_ATT_SCAN=1` | "Reload from AirPods" also reads ATT handles 0x0001-0x0060 and logs them (find where a firmware keeps a value; diff two runs) |

Settings (last audiogram / adjustments) are stored in
`$XDG_CONFIG_HOME/openaircare/` (defaults to `~/.config/...`).

## Raspberry Pi 5

Works the same; build natively on Raspberry Pi OS (64-bit, Bookworm or
later). The default Wayland session uses the V3D Vulkan driver; if the window
stays black try `MAKEPAD_GPU=gl`. On the X11 (openbox) session Makepad picks
the OpenGL ES 3.1 V3D backend by itself. A clean `--release` build takes
about 5 minutes on a Pi 5 (8 GB); the debug build of makepad about 10.

## Troubleshooting

- **`the AirPods rejected this host's pairing key` / `Invalid exchange (os error 52)`** -
  the buds no longer hold a link key for this host (HCI status `PIN or Key
  Missing`). This happens after a reset, after pairing them to another
  computer, or occasionally when the host's Device ID changes. Run
  `bluetoothctl remove XX:XX:...`, put the AirPods in pairing mode (lid open,
  hold the case button until the light blinks white) and pair again.
- **BlueZ shows the AirPods as connected only for a few seconds** - normal
  on hosts without a working A2DP/HFP setup (e.g. a bare Raspberry Pi): the
  audio profiles fail and BlueZ drops the link. The app does not need that
  link; it pages the buds itself when opening the AACP channel, and retries
  with back-off (2 s .. 30 s) while "Reconnect automatically" is on.
- **Sliders / audiogram have no effect** - the buds ACK every write of
  handle `0x2A` but silently discard it unless header byte 1 is `0x02`
  (a never-configured AirPods Pro 3 reads back `02 00 60 00`). The app forces
  `02 xx 64 00`; "Reload from AirPods" on the Audiogram tab reads the blob
  back so you can verify what the buds actually stored. Adjustments are only
  audible while Hearing Aid is on, in Transparency mode, with the buds in
  your ears. Verified on AirPods Pro 3: a stem swipe changes exactly the
  amplification fields the sliders write (offsets 36 / 84), and a value
  written by the app is applied immediately.
- **The Hearing Aid switch does not "confirm"** - AirPods Pro 3 on firmware
  8A apply `0x2C`/`0x33` but never echo them (older firmware does). The app
  therefore shows the value it sent; the buds report the real, persisted
  state in their initial dump on the next connect (Disconnect / Connect on
  the Status page to double check). The first enable also flips the
  "enrolled" flag on the buds permanently, like the Android app does.
- **`Connection refused` on PSM 31** - the `DeviceID` line is missing, the
  bluetooth service was not restarted, or the AirPods were not re-paired.
  If it *used* to work in this session, the buds are still holding the ATT
  slot of a previous client (another script, a killed app): the app retries
  PSM 31 five times, and if that is not enough, bounce the link once with
  `bluetoothctl disconnect XX:XX:... && bluetoothctl connect XX:XX:...`; the
  app reconnects on its own.
- **AirPods disconnect after a while when spoofing the Device ID** - known
  upstream quirk (the buds expect more Apple-specific traffic). The app
  reconnects automatically; when you are done configuring, remove the
  `DeviceID` line again.
- **`Permission denied` creating the L2CAP socket** - some distros restrict
  raw Bluetooth sockets; run once with `sudo` to confirm, then either add your
  user to the `bluetooth` group or `setcap cap_net_raw,cap_net_admin+eip` on
  the binary.
- **Wrong device picked** - the app connects to the first *connected* device
  advertising the AACP UUID `74ec2172-0bad-4d01-8f77-997b2be0722a`. Use the
  MAC field on the Status page to force a specific one.

## Shoutout
Librepods - check out their repo https://github.com/librepods-org/librepods
