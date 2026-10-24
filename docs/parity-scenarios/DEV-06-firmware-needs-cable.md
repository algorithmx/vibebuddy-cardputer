# DEV-06: Firmware actions need the cable

Category: Device tab · Platform observed: Mac app (`app/`)

## Where it lives

- `app/Sources/VibeBuddy/SettingsView.swift` (`DeviceView.onCable`)

## Initial state (input)

- The box is linked over Wi-Fi or the Relay (`device.network` true), and an update is available.

## Trigger

Settings → Device.

## Workflow (Mac app)

1. Instead of Update: "Plug in the cable to update the box's firmware."
2. "Flash from file…" and Retry are disabled; the daemon would refuse a flash over the network anyway.

## Final state (output)

- No flash is offered until the cable is in.

## Check on Linux

- [ ] Same rule? (platform-parity.md and docs/app.md say both apps do this.)

## Linux result

- Status: not checked
- Checked on: (date, commit, distribution and desktop)
- Evidence: (test, log, screenshot or code reference)
- Notes:
