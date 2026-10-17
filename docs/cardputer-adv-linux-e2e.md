# End-to-end test guide: Cardputer-ADV with the Linux desktop app

A manual test run with a real M5Stack Cardputer-ADV on USB, the real `vibebuddyd` under systemd, and the real Linux
app (`vibebuddy-desktop`) on a real desktop. It covers what the headless suite in
[`linux-host-testing.md`](linux-host-testing.md) can't: native USB discovery, the iced window, the tray, desktop
notifications, systemd, sound, keys and the compact 240×135 screen.

Written 2026-10-17 against branch `claude/cardputer-adv-port` (`9acc0f1`), for the bench this branch was developed on:
Ubuntu 22.04 GNOME on Wayland, the `70-vibebuddy.rules` udev rule installed, the user not in `dialout`.

## What is already established, and what this run is for

| Area | State before this run |
|---|---|
| Linux screenshot over USB with the Adv | User-confirmed 2026-10-17 (Ubuntu GNOME/Wayland) |
| Adv boot, ES8311 codec, speaker (focus-end line), K0 and K1 | Confirmed on the unit (`references/cardputer-adv/device-specs.md`) |
| K2 on the Adv | Never pressed against a computer |
| Daemon-failure notification (G4 P0) | Headless only; real systemd and notification server pending |
| K2 window focus on Sway/X11 (G4 P1) | Synthetic listings only |
| Adv over Wi-Fi, LAN or Relay | Not viable as built: 38,848 bytes free internal RAM at boot, before Wi-Fi starts |
| Adv board identity | The Adv firmware does not print `BOARD cardputer-adv`, so the daemon takes it for a box |

Each case below says what to expect from the current code, including the places where the expected result is a known
gap. A case "passes" when what you see matches **Expect**, even when that is a gap; report anything else.

## 0. Safety rules: read before plugging in

1. **Never accept a firmware update or "Flash VibeBuddy firmware" from the app.** Released firmware is built for the
   ALIENTEK box (16 MB flash, other pins). The Adv doesn't report its board, so the daemon's guard
   (`daemon/src/updates.rs`, `offer_to_box`) doesn't stop the offer. A dirty Adv build of 0.5.0 (`…-dirty`) is
   offered 0.5.0 as soon as update checks are on. The same thing once left a devkit with no screen, speaker or keys.
   Keep **Check for updates** off (the default in a source build), and if an **Update to …** button appears anyway,
   record it as a finding and don't click it.
2. **Flash the Adv with `tools/flash.sh` only** (`BOARD=cardputer-adv`). The app's own flasher sets 16 MB SPI
   parameters in the ROM (`daemon/src/rom_flasher.rs`, `prepare_flash`). Case E8 tries it with an Adv package, and
   only once you can recover with esptool.
3. **Wi-Fi on the Adv is exploratory** (case E6). It may run out of RAM. Keep the recovery steps in section 5 to hand.
4. The factory image is backed up at `~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.bin`. Check it is there
   before you erase or flash anything.
5. `install.sh` runs `vibebuddy-hook install`. That writes hooks into the user-level config of every agent it finds
   (Claude Code, Codex, OpenCode, Copilot CLI, Pi). Back up `~/.claude/settings.json` and `~/.codex/` first if you care
   about their exact contents.

## 1. Bench setup

### 1.1 Hardware and session

- The Cardputer-ADV, and a USB-C cable that carries data. Plug it into the computer directly, not through a hub,
  for the first pass.
- The default session: **Ubuntu (GNOME on Wayland)**. For the K2 window-focus cases you also need an
  **Ubuntu on Xorg** session (pick it from the gear on the login screen).
- Sound on, at a volume you can hear the box's own speaker over.

### 1.2 Host tools

Already on this machine: `jq`, `notify-send`, `pw-play`/`paplay`/`aplay`, `gnome-terminal`. Missing, so install them:

```sh
sudo apt install tmux wmctrl python3-serial pipx
pipx install esptool          # tools/flash.sh calls `esptool`; or set ESPTOOL="python3 -m esptool"
cargo install espup --locked && espup install   # writes ~/export-esp.sh; tools/build-firmware.sh sources it
cargo install espflash --locked                 # build-firmware.sh uses `espflash save-image`
```

The firmware toolchain is needed only if you build the Adv firmware on this machine. If the Adv already shows the
idle pet and `READY`, and you don't need the fast-clock build, skip 1.3.

### 1.3 Build and flash the Adv firmware

Commit or stash first, so the build ID isn't `-dirty` (safety rule 1).

```sh
BOARD=cardputer-adv tools/flash.sh /dev/ttyACM0
```

It builds into `firmware-rs/device/build-cardputer-adv/` and writes the bootloader, partition table, app (ota_0) and
a blank otadata. It leaves `nvs`, `voices` and `link` alone, so the box keeps its key, pairings, volume and
Character. If esptool can't connect, hold **K0 (G0)** while you plug in the cable (download mode, dark screen) and
run it again. Unplug and replug to boot afterwards.

For the Pomodoro cases (section H), build the fast-clock variant once all the other cases are done. Every Pomodoro and
Leisure duration is divided by 60, so a 25 min focus lasts 25 s and a 5 min break 5 s:

```sh
FAST_CLOCK=1 BOARD=cardputer-adv tools/flash.sh /dev/ttyACM0
```

Afterwards, flash the normal build again.

### 1.4 Install the computer side

Use the installer from the checkout. It builds release binaries, installs the systemd user unit and the Character
packs, and sets up autostart. The outage and restart cases need the real unit.

```sh
packaging/linux/install.sh
systemctl --user status vibebuddyd      # active (running)
ls ~/.local/share/vibebuddy/voices      # robot.bin plus the character packs (ada, dave, hank, kai, luna, mei, wanwanxiaohe)
```

Without `install.sh`, the Character tab lists no packs and Advanced → Restart daemon has no unit to restart.

Start the app: `~/.local/bin/vibebuddy-desktop --settings`. A plain launch starts in the tray only. A second launch
hands off to the running copy.

For a code change during the run: rerun `install.sh`. It stops the daemon and app, reinstalls and restarts the
daemon. Then start the app again.

### 1.5 Port access

```sh
ls -l /dev/serial/by-id/            # usb-Espressif_USB_JTAG_serial_debug_unit_…-if00 -> ../../ttyACM0
getfacl /dev/ttyACM0 | grep user:   # user:dabajabaza:rw-  (granted by uaccess, no group needed)
```

If there's no `user:` ACL line, run `sudo udevadm control --reload && sudo udevadm trigger`, then replug.

### 1.6 Watching what happens

Keep three terminals open during the run:

```sh
journalctl --user -u vibebuddyd -f -o short-precise           # daemon log
watch -n1 'curl -s 127.0.0.1:7331/v1/status | jq -c "{c:.device.connected, port:.device.port, bridge:.device.bridge, net:.device.network, fw:.device.firmware_version, build:.device.firmware_build, board:.device.unsupported_board, foreign:.device.foreign_firmware, vol:.device.volume, voice:.device.voice, paired:.device.paired, wifi:.device.wifi_network, op:.operation}"'
watch -n1 cat ~/.config/vibebuddy/config.json                 # settings as the daemon saved them
```

The settings file has four keys: `voice`, `notify_link`, `check_updates` (`null` means "follow the build", which is
off in a source build) and `relay`. Every toggle below should change exactly one of them.

To see what the box itself prints (for example `MEMORY INTERNAL`), stop the daemon first, because it holds the port:

```sh
systemctl --user stop vibebuddyd
python3 -m serial.tools.miniterm /dev/ttyACM0 115200    # opening the port resets the board; Ctrl+] quits
systemctl --user start vibebuddyd
```

Before you start, open Advanced → **Export diagnostics…** and keep the file, so there's a baseline.

## 2. Test cases

Each case gives **Steps** and **Expect**. Record pass/fail and anything odd in the template in section 4.

### A. Discovery and link

**A1. First plug-in.** Steps: daemon running, app open on Device. Plug in the Adv.
Expect: within about 3 s, Device → Link reads `/dev/ttyACM0 · native USB`, and status shows `bridge:false`.
Box firmware reads `0.5.0` with its build ID, the same as `firmware-rs/device/build-cardputer-adv/build.txt`.
The box shows the USB plug icon at the top left and the idle pet. In the status, `board` is `null` (known gap) and
`foreign` is `false`.

**A2. Pairing over the cable.** Expect: without any action, `paired:true` and Device → Paired computers lists this
computer as "This computer", with no Remove button.

**A3. Unplug and replug.** Steps: unplug for 5 s, then plug back in.
Expect: Link goes to "Box not found", with the cable and permission hints, and back to native USB after the replug.
Volume, Character and pairing are the same as before. The daemon log shows no write errors after the reconnect.

**A4. Box-gone notification.** Steps: General → "Notify me when the box disconnects or the daemon fails" on. Unplug
for 40 s.
Expect: one GNOME notification at about 30 s: "VibeBuddy hasn't seen the box for 30 seconds. Check the USB cable."
It comes from the daemon (`link_alert.rs`), not the app. Replug, then unplug for 40 s again: one more notification.
Repeat with the toggle off: no notification.

**A5. Port pin.** Steps: `systemctl --user edit vibebuddyd`, add `[Service]` and
`Environment=VIBEBUDDY_SERIAL_PORT=/dev/ttyACM9`, then restart the daemon.
Expect: Device shows the warning "Only looking at /dev/ttyACM9, set by VIBEBUDDY_SERIAL_PORT", and the box is not
found. Remove the override (`systemctl --user revert vibebuddyd`), restart, and the box comes back.

**A6. Permission denied (optional).** Steps: `sudo setfacl -b /dev/ttyACM0` and restart the daemon.
Expect: the log says `Permission denied`, and Device shows the "Permission denied … restart the computer once" hint.
Replug the box to restore the ACL.

**A7. Download mode looks like foreign firmware.** Steps: hold K0 while plugging in.
Expect: the screen stays dark. The app shows "The box isn't running VibeBuddy firmware" only if a firmware package
has been downloaded, which it hasn't in a source build. Otherwise you'll see the "Box not found" or "Looking for
firmware…" text. **Don't click Flash** (safety rule 1). Replug without K0 to boot normally.

### B. General tab

**B1. Status lines.** Expect: the device, mode and today lines at the top match the tray menu's (G1). With the box
unplugged, the device line says so.

**B2. Launch at login.** Steps: toggle it off.
Expect: `~/.config/autostart/vibebuddy.desktop` becomes an entry with `Hidden=true`. Toggle it on: the entry gets
`Exec=` pointing at the running binary. Log out and back in with it on: the tray icon appears and the window doesn't.
With it off: the app doesn't start.

**B3. Notify toggle.** Expect: `notify_link` in `config.json` flips. While the daemon is down the toggle is greyed
out. Behaviour is tested in A4 and F4.

**B4. Relay toggle.** Expect: `relay` flips, and the box isn't affected (it is on USB). Over USB the Relay isn't used.
Its real behaviour can't be tested with the Adv.

**B5. Check for updates.** Steps: leave it off at first.
Expect: the summary reads "Off", Check now is greyed out, and Device → Latest firmware says "Update checks are off
(General)". Then turn it on and press **Check now**. Expect "Up to date · checked …" or "VibeBuddy 0.x is available"
with the install.sh hint. On Device, if an **Update to 0.5.0** button appears because the build is dirty, or any
other version is offered to the Adv, that is the known board-identity gap: record it, **don't click it**, and turn
the toggle off again.

**B6. Releases and Report a Problem….** Expect: Releases opens the GitHub releases page in the browser. Report a
Problem… opens a new-issue page with the diagnostics summary (both builds, voice, OS) filled in, and no logs.

**B7. App version.** Expect: the "App" row reads `0.5.0`.

**B8. Language.** Steps: pick 简体中文.
Expect: a "Restart VibeBuddy to change the language?" prompt. With the box connected and a Chinese Character
installed, a checkbox offers "Also switch the box's voice to …". **Later** keeps the app in English. **Restart now**
restarts the app in Chinese on the same tab. With the checkbox ticked, the box gets the Chinese Character once it
answers (watch `op` in the status). Check that every tab is translated, and look for text that is clipped on the
compact layout. Switch back to System or English the same way. Unplug the box and repeat: the prompt says the box's
Character stays as it is.

### C. Character tab

**C1. Volume slider.** Steps: drag it to 30 and release.
Expect: the box plays nothing on the drag, the status `vol` becomes 30, and the value persists across replugging the
box (A3) and restarting the daemon. The slider goes from 20 to 100 in steps of 5. It is inactive while the box is
unplugged or a write is running.

**C2. Play a line on the box.** Expect: the box says a line at the current volume through the Adv speaker. Check it
is clear, with no clipping at 100 and still audible at 20.

**C3. Volume from the box.** Steps: on the box, hold K1 for 1 s to open the menu, press K1 to reach VOLUME, then
press K0.
Expect: the box steps through 20 → 35 → 50 → 65 → 80 → 100 and plays "All done!" at each level. The app's slider
follows each step within about 1 s. K2 closes the menu.

**C4. Preview on the computer.** Steps: press ▶ on any Character.
Expect: its lines play through the computer's speakers (`pw-play`). ■ stops them, and ▶ on another Character switches
to it. Previews are greyed out while the box is being written.

**C5. What the buddy calls you.** Steps: pick a form of address for English and one for Chinese.
Expect: the choice stays after the app restarts. The next Character written to the box uses the pack variant for
that form (C6), so the line the box says after writing includes the form.

**C6. Write a Character over native USB.** Steps: press **Use** on a small pack (dave, 440 KB), then on the largest
(wanwanxiaohe, 798 KB).
Expect: a progress line, with `op` running in the status. When it finishes, the box says a line in the new voice,
the card shows "In use", and `config.json` → `voice` is set. Record how long each write takes. Native USB should take
well under the "few minutes" quoted for the UART bridge. Every pack fits in the Adv's 1.75 MB voices partition.

**C7. Interrupted write.** Steps: start a write, then unplug at about 50%.
Expect: the operation fails with "Didn't finish, so the box keeps its built-in voice. Reconnect the cable and click
Use again." Replug: the box boots normally and speaks in the built-in voice. Use again: the write completes.

**C8. Back to the robot.** Steps: **Use** on the VibeBuddy (Default) card.
Expect: the robot face with the built-in voice's lines, and "In use".

**C9. Your own character.** Steps: Choose images… and pick one figure on a white background, or four (normal, eyes
closed, happy, sad). Choose a Character for "Voice and lines from", then press Use.
Expect: the frames show in the app on a black background. After the write, the box shows the figure in place of the
robot. Check it fits and reads well at 240×135 (Device → Refresh). Pick an image with a busy background: the app
says "No figure found…". **Copy a prompt** puts a prompt for drawing a figure on the clipboard.

### D. Agents tab and events

**D1. Agent list.** Expect: each agent on this machine is listed with its state. "… wasn't found on this computer"
means the agent isn't installed, "Not set up" means no hooks, and "Waiting for the first event…" or "Last event …"
means hooks are installed. After `install.sh`, the agents you have should be set up already.

**D2. Remove, then Connect.** Steps: on Claude Code, press **Remove**.
Expect: a "Will change ~/.claude/settings.json" box with `-` lines in red. **Cancel** changes nothing (check the
file's modified time). Remove → **Write**: the hooks are gone and the row says "Not set up". **Connect** → Write: `+`
lines, and the row says "Waiting for the first event…". **Repair** with nothing to fix: "Nothing to change." and
Write is greyed out.

**D3. Codex trust hint (if Codex is installed).** Expect: after Write, the hint to trust the config in `/hooks`. If
you leave it untrusted, the row warns that the config changed and no event has arrived.

**D4. A real session.** Steps: in a terminal, run `claude` in a git project and give it a task that takes a while and
asks for permission once.
Expect, on the box, at 240×135: a working card with the session or branch title, then the needs-input screen and line
("Hey, I need you for a sec."), then working again once you answer, then done with "All done!". About 5 s later it
returns to idle, or to the other open cards. Agents → Claude Code reads "Last event <time>". The idle screen's stats
lines count the task. Check that long titles are cut off cleanly and that two concurrent sessions show as two cards.

**D5. Synthetic events.** These test the screens without an agent:

```sh
ev() { curl -s -H 'content-type: application/json' --data "$1" http://127.0.0.1:7331/v1/events; echo; }
ev '{"version":1,"event":"task.start","title":"ALPHA"}'
ev '{"version":1,"event":"agent.input_required","title":"ALPHA"}'
ev '{"version":1,"event":"task.error","title":"ALPHA"}'
ev '{"version":1,"event":"task.done","title":"A VERY LONG TITLE THAT WILL NOT FIT ON THE COMPACT SCREEN"}'
ev '{"version":1,"event":"agent.idle"}'
ev '{"version":1,"event":"buddy.say","occasion":"greeting_morning"}'
```

Expect: each one draws the right screen, and the needs-input, failed and done events play their line once. Grab each
screen with Device → Refresh. Sending input_required while the box menu is open closes the menu (H3).

### E. Device tab

**E1. Rows.** Expect: Link `/dev/ttyACM0 · native USB`, Box firmware with version and build, and Latest firmware
"Update checks are off (General)". There is no "This is a … board" line, because the Adv doesn't report its board
(known gap).

**E2. Make the box blink.** Expect: the Adv's backlight (G38) flashes. The same button is on the first-run guide's
Find the box page.

**E3. Box screen.** Steps: Refresh on the idle screen, a working card, the menu and a Pomodoro screen.
Expect: a crisp 240×135 image that matches the panel: same content and colours, nothing missing at the edges (the
panel offsets are 40/53). **Save image** writes a PNG. Check `file saved.png` reports 240 x 135, and press Refresh
while one is already running: nothing breaks.

**E4. Paired computers.** Expect: one row, "This computer". If a second Linux or Mac computer with VibeBuddy is
available, plug the Adv into it once, then back into this one. Two rows appear, and the other one has **Remove**.
Remove it and it disappears. The box's menu → COMPUTERS shows the same count.

**E5. Take over (needs the second computer).** With the Adv on the other computer's cable, this one shows "The box is
plugged into <name>." with no Take over button, because it is on USB there. Over the network it would offer Take
over, but the Adv can't do that as built.

**E6. Wi-Fi (exploratory; read section 5 first).** Steps: Scan.
Expect one of two outcomes: a list of nearby 2.4 GHz networks with their dBm, or a failure or reboot. Either way,
record the daemon log and what the box shows. If the scan works, Join a test network: "Joining… This can take up to
15 seconds.", then "Joined · <address>" or one of the specific failures (wrong password, not found, security).
Then **Forget network**. Check the box's `MEMORY INTERNAL` line before and after (section 1.6). A crash or boot loop
here is the expected RAM limit, not a desktop bug; recover with section 5.

**E7. Several ESP32-S3 boards (optional).** Steps: plug in a second ESP32-S3 on native USB (the ALIENTEK box, or a
devkit), after forgetting the known box (`rm ~/.local/state/vibebuddy/box-usb-serial`, then restart the daemon).
Expect: "Several devices are plugged in…", with each port and USB serial and a **This is the box** button. Pick the
Adv: it connects, and is remembered after a daemon restart.

**E8. Flash from file… with an Adv package (optional, risky).** Only do this with esptool working and section 5 to
hand. Steps:

```sh
cd firmware-rs/device/build-cardputer-adv && zip /tmp/adv-fw.zip bootloader.bin partition-table.bin vibebuddy-fw.bin build.txt version.txt
```

Then Device → Flash from file… → `/tmp/adv-fw.zip`.
Expect: a confirm box listing the package and what the box runs now. Cancel changes nothing. Press Flash and record
everything: progress, any replug or K0 prompts, and whether the Adv boots afterwards with its voice and pairing kept.
The flasher tells the ROM the flash is 16 MB (safety rule 2). A failure here is a finding against the in-app flasher
on the Adv. Recover with `tools/flash.sh`.

### F. Advanced tab and the daemon (G4 P0 acceptance)

**F1. Rows.** Expect: "daemon · Running · <build>", and "Config file: ~/.config/vibebuddy/config.json".

**F2. Restart daemon while it runs.** Expect: the app shows down for a moment, then running again. `journalctl` shows
a stop and a start. The box reconnects within a few seconds. There's no outage notification, because the restart is
shorter than the 30 s grace.

**F3. Restart daemon while it is down, without the tray.** Steps: `systemctl --user stop vibebuddyd`, wait 5 s, then
press **Restart daemon** in the window.
Expect: the button works while the daemon is down, the daemon starts, and the box comes back. Then break it on
purpose: `systemctl --user mask vibebuddyd` (stop it first), press Restart. Expect a visible error notice, not
silence. Unmask afterwards: `systemctl --user unmask vibebuddyd && systemctl --user start vibebuddyd`.

**F4. Daemon-failure notification.** With the Notify toggle on:
- *Intentional stop:* `systemctl --user stop vibebuddyd` and wait 70 s. Expect no notification, because an inactive
  unit suppresses it and is rechecked after another 30 s and still inactive.
- *Crash:* `systemctl --user kill -s KILL vibebuddyd`. systemd restarts it after 2 s. Expect no notification.
- *Real failure:* make the daemon fail on start. Run `systemctl --user edit vibebuddyd`, add `[Service]`,
  `ExecStart=` and `ExecStart=/bin/false` (the empty line clears the original), then
  `systemctl --user restart vibebuddyd`. Expect exactly one GNOME notification about 30 s after the app loses the
  daemon. "The daemon stopped and didn't come back. Restart it from the tray icon or on the Advanced tab." No second one while it stays down. Then
  `systemctl --user revert vibebuddyd && systemctl --user reset-failed vibebuddyd && systemctl --user start vibebuddyd`.
  The app recovers, and a later failure notifies again.
- *Stop, then fail:* stop the service, wait 40 s (suppressed), then apply the `/bin/false` override and start it.
  Expect a notification within about 30 s more.
- Repeat the real failure with the toggle off: no notification.

**F5. Show daemon log.** Expect: it runs `xdg-terminal-exec journalctl --user -u vibebuddyd -f`. Ubuntu 22.04 doesn't
ship `xdg-terminal-exec`, so expect the notice "cannot run xdg-terminal-exec: …". If you see it, record it as a
finding for Ubuntu/GNOME: there is no fallback to `gnome-terminal`.
`sudo apt install xdg-terminal-exec` (if your release has it) should then open a terminal following the journal.

**F6. Open logs folder.** Expect: Files opens `~/.local/state/vibebuddy` (the hook's log, `box-usb-serial`, stats). The
daemon itself logs to the journal.

**F7. Export diagnostics….** Expect: a save dialog, then "Saved to …". The file has both build IDs, the voice, the OS
and `config.json`, and no prompts, replies or Wi-Fi password.

**F8. Stale stream (optional).** `kill -STOP $(pidof vibebuddyd)` for 60 s.
Expect: the app reports the daemon down within about 45 s (the stream inactivity limit), rather than showing stale
"running" forever. Then `kill -CONT` it, and the app recovers.

### G. Tray and app lifecycle

**G1. Tray icon and menu.** Ubuntu ships the AppIndicator extension, so the face icon appears in the top bar.
Expect: the menu shows device, mode and today lines, then Settings…, Check for Updates…, Report a Problem… and Quit.
The face changes with the box's state (D5). Settings… opens or focuses the window. Check for Updates… opens General
and runs a check, so it is subject to B5.

**G2. Tray restart while down.** Stop the daemon. Expect: the device line offers a restart, and clicking it starts
the daemon (the same systemd path as F3).

**G3. Quit leaves the box online.** Steps: tray → Quit.
Expect: the app exits, the daemon keeps running, the box stays linked and still gets events (D5). Starting the app
again reconnects to the same state.

**G4. Closing the window.** Expect: the app stays in the tray. `vibebuddy-desktop --settings` reopens the window.

**G5. No tray host.** Steps: `gnome-extensions disable ubuntu-appindicators@ubuntu.com`, then restart the app with a
plain launch, without `--settings`.
Expect: the settings window opens about 10 s after start. Re-enable the extension: the icon comes back, and the app
keeps working.

**G6. First-run guide.** Steps: quit the app, move `~/.config/vibebuddy/` aside, start the app, and move it back
afterwards.
Expect: Welcome → Find the box (Make it blink works, the Link row is live) → Pick a character (writes, as in C6) →
Wi-Fi (offers to put the box on Wi-Fi; choose **Skip** on the Adv) → Done → Get started opens the tabs.

### H. Box keys against the desktop

**H1. K1 (`;`).** Expect: switches between On duty and Pomodoro. The General mode line and the tray follow within
about 1 s.

**H2. K0 (G0 / BOOT).** In Pomodoro: start, pause and resume the focus timer, and the mode line follows. **K0 long:**
gives up the phase. Use the fast-clock build here: a focus ends after 25 s with the "ding dong" line, then the break
runs for 5 s. Check the today line counts the focus, and that the count survives a daemon restart.

**H3. K1 long (1 s): the menu.** Expect: it opens while K1 is still held. K1 steps through the rows (STOP FOCUS only
during a phase, VOLUME, MUTE, COMPUTERS, NETWORK, STATUS), K0 acts, and K2 or 30 s idle closes it. STATUS shows the
firmware and app builds, USB as the link and the voice pack, and should match the app's Device tab. NETWORK shows
LINK `USB`, WIFI `OFF`, and RELAY dimmed. Check every row and hint fits on 240×135 (Device → Refresh).

**H4. K2 long (`.` held): mute.** Expect: MUTE shows on the screen, and events (D5) are silent. C2 "Play a line"
still plays, or doesn't: record which. Unmute with K2 long. Mute stays on the device and isn't in the app.

**H5. K2 short: back to the agent.** This is the first time K2 is tested on the Adv. Run it in three setups, each with
a Claude Code session that has sent at least one event:
- *GNOME Wayland, plain terminal:* expect nothing visible. The daemon log shows the press, then a logged failure that
  no supported window backend was found. This is the documented Linux gap.
- *GNOME Wayland, inside tmux:* run `claude` inside a tmux pane, then select another pane or window. Expect K2 to
  switch the tmux client back to the agent's pane. The terminal window itself isn't raised.
- *Ubuntu on Xorg with `wmctrl`:* log in to the Xorg session. First check the daemon sees X11:
  `systemctl --user show-environment | grep -E 'DISPLAY|XDG_SESSION_TYPE'`. Minimise or cover the terminal and press
  K2. Expect the terminal running the agent to come to the front (`wmctrl -i -a`). With two sessions, one waiting
  for input and one done, K2 picks the one waiting for input. If the daemon started before the session's
  environment was imported, it may still choose Hyprland. Record that, run
  `systemctl --user restart vibebuddyd`, and try again (G4 finding 3.4).
Also check that K2 on a GitHub Actions card opens the run page through `xdg-open`, if you have a repo with recent runs
and `gh` logged in.

### I. Robustness

**I1. Suspend and resume** the laptop with the box plugged in. Expect: the box is back on native USB within a few
seconds of resume, with no notification storm.

**I2. Reset button on the box (EN).** Expect: the same as A3. The daemon re-reads the build, volume and voice.

**I3. Hub and other ports.** Repeat A1 and C6 through a USB hub and on another port. Record any write errors or slow
receipts in the journal.

**I4. Long soak.** Leave the box connected with a real agent working for 1–2 hours. Expect: no disconnects, no stuck
screen, and the stats keep counting. Afterwards, take a `MEMORY INTERNAL` reading (section 1.6) to check for heap
loss.

## 3. Cleanup

```sh
systemctl --user revert vibebuddyd; systemctl --user reset-failed vibebuddyd; systemctl --user restart vibebuddyd
gnome-extensions enable ubuntu-appindicators@ubuntu.com
```

Put back any `config.json`, agent config or factory files you moved aside, and flash the normal (not fast-clock) build.

## 4. Recording results

One row per case, plus a short note for each failure:

| Case | Session | Result | Notes (what you saw, time, log excerpt) |
|---|---|---|---|
| A1 | GNOME/Wayland | pass | 2.1 s to native USB |

For each failure, keep: the time; `journalctl --user -u vibebuddyd --since "<time>"`; the status JSON
(`curl -s 127.0.0.1:7331/v1/status | jq .`); a Device → Save image of the box; and Advanced → Export diagnostics.
Record the outcomes for the open items in [`linux-host-testing.md`](linux-host-testing.md) and
[`platform-parity.md`](platform-parity.md) under a dated heading, the way the screenshot acceptance was recorded.

## 5. Recovering the Adv

- **Box stuck or boot-looping after Wi-Fi (E6):** first try the box's menu → COMPUTERS → NEW ID. That forgets the Wi-Fi
  network, pairings and Relay, and the box pairs again on the cable. If the menu can't be reached, hold K0 while
  plugging in and erase the link partition. The box gets a new key and pairs again:
  `esptool --chip esp32s3 --port /dev/ttyACM0 erase_region 0x7d2000 0x2000`.
- **App won't boot after a flash (E8):** download mode (hold K0 while plugging in), then
  `BOARD=cardputer-adv tools/flash.sh /dev/ttyACM0`.
- **Everything else:** restore the factory image with
  `esptool --chip esp32s3 --port /dev/ttyACM0 write_flash 0x0 ~/cardputer-adv-backup/factory-flash-8MB-2026-10-16.bin`
  (check its SHA-256 first), then flash VibeBuddy again.
