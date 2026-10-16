# G4: plan to bring the Linux computer side up to the Mac

Second version, 2026-10-17, after re-checking every gap against the code (`docs/platform-parity.md`, "Corrections").
It lists what to change, in what order, and how to tell it worked. It closes the gaps in the review and nothing more.
The differences marked "by design" in the review stay, and get documented.

Rules from AGENTS.md that apply to every item: UI strings are English in source with their zh-Hans translation in
`app/Localization/zh-Hans.lproj` (the Linux app shares the table; `tools/check-localization.py` checks it). `README.md`
and `README.zh-CN.md` change together, section for section.

## P0: the daemon-failure notification on Linux

- **Gap:** the Linux toggle reads "Notify me when the box disconnects or the daemon fails", but nothing on Linux
  notifies when the daemon fails. On the Mac the app does it, when its supervisor gives up after three failed starts
  (`AppModel.swift`, `onGiveUp`), following the toggle and defaulting to on when no config can be read.
- **Change:** the same in the Linux app (`desktop/src/main.rs`). It already knows when the daemon is gone (the tray's
  daemon-down face) and already sends notifications (`notify_new_firmware`, through `notify-send`). When the daemon has
  been unreachable for 30 s, send one notification, then none until the daemon is back. Skip it when systemd reports the
  unit as `inactive` (`systemctl --user is-active vibebuddyd`), which means someone stopped it on purpose. Follow the
  last known notify toggle, on when unknown, as the Mac does.
- **Why 30 s:** systemd restarts the daemon every 2 s and gives up after the default start limit (5 starts in 10 s),
  so a daemon still gone after 30 s isn't coming back on its own.
- **String:** a Linux wording of the Mac's message, e.g. "The daemon stopped and didn't come back. Restart it from the
  tray icon or on the Advanced tab.", with its zh-Hans translation.
- **Tests:** the timing rule as a pure function with unit tests, like `LinkAlert` in `daemon/src/link_alert.rs`. By hand:
  kill the daemon so it crash-loops (or `systemctl --user stop` it, which must not notify), and check one notification
  after 30 s and a reset once it is back.
- **Rejected alternative:** a systemd `OnFailure=` unit. It needs the session bus address to reach the desktop, a change
  in two packagings, and it would ignore the app's toggle. The Mac model is app-side; this keeps both the same.

## P1: K2 window focus outside Hyprland

- **Gap:** focus works only with `hyprctl` (`daemon/src/source_opener.rs`, `focus_window`). Elsewhere K2 only switches
  a tmux pane, if there is one.
- **Change:** two more backends behind `focus_window(pids)`, chosen by the session, with Hyprland first:
  - **Sway** (`SWAYSOCK` set): read `swaymsg -t get_tree` (JSON), take the window whose `pid` is the nearest of the
    agent's ancestors, focus it with `swaymsg '[con_id=N] focus'`. `swaymsg` ships with sway.
  - **X11** (`XDG_SESSION_TYPE=x11`, or `DISPLAY` without `WAYLAND_DISPLAY`): `wmctrl -l -p`, the same pid match,
    then `wmctrl -i -a <id>`. `wmctrl` is a new optional dependency: list it in the pacman package's optional
    dependencies and in the README's Linux section.
- **Tests:** keep the parsing in pure functions, with fixtures of the sway tree and of `wmctrl -l -p` output, so CI
  (Ubuntu, no desktop) runs them. By hand: an agent in a terminal, K2 brings that terminal forward on sway and on X11.
- **Known limit, documented:** a terminal that runs all its windows in one process (for example gnome-terminal or a
  single-instance kitty) owns several windows under one pid, so K2 may pick the wrong one. Hyprland has the same limit
  today.
- **Not covered, documented:** GNOME and KDE on Wayland have no command to focus a window by pid, so K2 stays a tmux
  pane switch there.

## P2: K2 into the Claude and Codex apps on Linux (investigate first)

- **Gap:** on the Mac, K2 opens the exact session or thread. On Linux the hook reports only process ancestors
  (`hook/src/surface.rs`) and never the desktop session ID (`hook/src/claude.rs`), so K2 at best focuses the app window,
  under Hyprland.
- **Step 1 (decides whether to build anything):** on Ubuntu with the Claude desktop beta and the ChatGPT/Codex preview
  installed, run a session in each and check: whether the hook's environment has `CLAUDE_CODE_HOST_SESSION_ID`;
  `xdg-mime query default x-scheme-handler/claude` and `x-scheme-handler/codex`; and whether
  `xdg-open 'claude://code/continue?session=<id>'` and `xdg-open 'codex://threads/<id>'` open the session.
- **Step 2, only if step 1 holds:** the hook reports the desktop session ID on Linux too when the variable is set, and
  the daemon tries `xdg-open` with the same link before falling back to window focus. Tests: the link choice per
  platform in `source_opener.rs`.
- **If step 1 fails:** document that K2 brings the app window forward (Hyprland, and P1's sway and X11), not the
  session.

## P3: say why there is no tray icon

- **Gap:** without a tray host the app opens its window after 10 s (`TrayHostCheck`) but doesn't say why. That happens
  on Fedora and upstream GNOME without the AppIndicator extension. Ubuntu preinstalls the extension; KDE and Omarchy have
  a tray.
- **Change:** document it in the README's Linux section (both languages). Only if users still miss it, add one line to
  the window when it opened for this reason; that line is Linux-only by nature (the Mac always has a menu bar).

## P4: documentation

- `README.md` and `README.zh-CN.md`, Linux section: K2 per desktop (Hyprland, then sway and X11 after P1; what GNOME
  and KDE Wayland do), the daemon-failure notification after P0, the tray note from P3, `wmctrl` as optional.
- `docs/app.md`: a short Linux section pointing to `docs/platform-parity.md` for the full table.
- `docs/platform-parity.md`: move each row as its item lands.

## Not planned

- A systemd `OnFailure=` alert (see P0).
- Self-updating on Linux: the package manager or `install.sh` stays the path; documented as by design.
- Terminal-tab focus on Linux, and Warp's deep link outside macOS: not requested, and the Mac's Automation permission has
  no Linux counterpart.
- Tests for the Mac app's UI: not a Linux gap, so out of this goal's scope.
- A line in the Agents tab explaining a failed K2: it would be a one-platform UI change for a logged condition; the docs
  cover it.

## Order and verification

Order: P0, then P1, with P4's README changes landing with each. P2 step 1 whenever a Linux machine with both apps is at
hand. P3 with the README pass.

| Check, on a real desktop | Hyprland | sway | X11 | GNOME Wayland | KDE Wayland |
|---|---|---|---|---|---|
| K2 focuses a terminal agent | works today | P1 | P1 | not covered (tmux only) | not covered (tmux only) |
| K2 opens a Claude or Codex app session | P2 step 1 | P2 step 1 | P2 step 1 | P2 step 1 | P2 step 1 |
| Daemon-failure notification | P0 | P0 | P0 | P0 | P0 |
| Tray icon, or the window with an explanation | tray | tray | depends on the panel | needs the extension (Ubuntu has it) | tray |
