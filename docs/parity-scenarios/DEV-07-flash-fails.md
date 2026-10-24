# DEV-07: A firmware flash fails

Category: Device tab · Platform observed: Mac app (`app/`)

## Where it lives

- `app/Sources/VibeBuddy/SettingsView.swift` (`DeviceView`, `OperationRow`)

Daemon API: `POST /v1/device/firmware`

## Initial state (input)

- A firmware flash is running.

## Trigger

The flash fails.

## Workflow (Mac app)

1. The row shows "Failed: {daemon message}".
2. Below it: "Before retrying, hold K0 on the box and replug the cable to put it in download mode." and Retry, enabled only on the cable.
3. Retry flashes the offered firmware again (DEV-05).

## Final state (output)

- The user can put the box in download mode and retry.

## Check on Linux

- [ ] Same failure text, K0 hint and Retry? (platform-parity.md: "retry with the K0 hint".)

## Linux result

- Status: not checked
- Checked on: (date, commit, distribution and desktop)
- Evidence: (test, log, screenshot or code reference)
- Notes:
