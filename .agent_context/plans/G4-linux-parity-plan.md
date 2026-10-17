# G4: plan to bring the Linux computer side up to the Mac

Third version, 2026-10-17. Integrates the first test-infrastructure wave and adds reflection checkpoints during
implementation. This revision is planning only: no implementation steps, test commands or desktop checks are run.

The feature scope remains P0–P4 from the second-pass review in `docs/platform-parity.md`. Differences marked
"by design" stay and get documented. The existing harness and its evidence are described in
`docs/linux-host-testing.md`; they are the starting point, not proof that the parity gaps are closed.

Rules from AGENTS.md that apply to every item: UI strings are English in source with their zh-Hans translation in
`app/Localization/zh-Hans.lproj` (the Linux app shares the table; `tools/check-localization.py` checks it). `README.md`
and `README.zh-CN.md` change together, section for section.
Keep user-visible Mac/Linux behavior aligned; document platform-specific behavior and its alternative.

## T0: establish the test baseline before feature changes

**Already built:** a PTY harness joining the real Linux daemon to the Cardputer-ADV firmware core, using the Adv's
production partition CSV; button, clock and reboot controls; wire fault injection; isolated config/state and
loopback services; recorded desktop-command stubs; and a headless consumer of the Linux app's status/tray model.
Reuse `tools/host_test_support.py`, `tools/test_linux_host.py` and `desktop/examples/host_status_probe.rs`.

**Historical evidence:** `docs/linux-host-testing.md` records 30 scenarios exercised: 29 passes and one expected
screenshot failure. The delayed/duplicate-receipt case was run separately after the recorded full run. This is
previous evidence, not a fresh baseline for the next implementation revision.

At the start of execution:

1. Run the complete host suite at one revision and record the revision, case counts, expected failures, duration
   and artifact location. Check that cleanup and isolation still work without a box or desktop session.
2. Confirm the workspace and separate Adv core baselines using the verification commands below. Keep the board
   features in separate builds; workspace tests select BOX through the daemon's dev-dependency.
3. Inventory the evidence boundaries below. Identify the smallest missing test seam for P0 and the missing command
   fixtures/environment controls for P1 before implementing those features.
4. Add a T0 reflection entry. If a baseline fails, distinguish a product regression, harness defect and unavailable
   environment; repair a harness blocker before relying on its results. Unrelated limitations stay explicit.

| Existing evidence | What it establishes | What needs additional work |
|---|---|---|
| Firmware core + PTY + real daemon HTTP/SSE | Device signals reach host parsing, aggregation, commands and status | Physical USB enumeration, permissions, DTR resets, native-USB timing and actual unplug/re-enumeration |
| Linux app status/tray probe | Real snapshot decoding and menu derivation | App stream reconnect handling, notification policy, command execution, tray detection and rendered UI |
| `hyprctl` command scenarios | Arguments, PID selection, dispatch fallback and failure recovery | Real compositor focus, including the environment inherited by the systemd daemon |
| Stub names for `swaymsg`, `wmctrl`, `systemctl`, `notify-send`, `xdg-open` | Places to attach controlled responses and inspect calls | Scenario-specific command/argument responses and assertions; their existence is not feature coverage |
| Firmware `advance` control | Device timers can advance without wall-clock waits | Host notification and receipt deadlines; use explicit timestamps or Tokio test time for those |

The PTY currently takes the daemon's conservative bridge-pacing path and reports `device.bridge=true`. Retain this
fact in test reports; passing it does not verify the Adv's native USB transport. Keep command stubs, environment
overrides and service changes confined to the harness's child processes and temporary directories.

### Known device gaps carried forward

- **240×135 screenshots:** retain the existing characterization test and expected failure while this behavior is
  untouched. If a later change fixes it, update the old-error characterization and remove the expected-failure
  marker together; require a real 240×135 PNG. Do not change the simulator to emit a BOX-size frame to get a pass.
- **Missing Adv board report:** synthetic `BOARD cardputer-adv` proves only the host parser path. It does not prove
  the device reports its identity or that an update offer targets the correct hardware. Track device reporting
  with the Cardputer work separately; do not invent a boot signal or expand G4 into firmware changes implicitly.

Neither gap blocks unrelated P0/P1 work. Any newly discovered gap affecting an item's acceptance must be recorded
and handled in that item's reflection; an unrelated fix becomes an explicit follow-up.

## P0: the daemon-failure notification on Linux

- **Gap:** the Linux toggle reads "Notify me when the box disconnects or the daemon fails", but nothing on Linux
  notifies when the daemon fails. On the Mac the app does it, when its supervisor gives up after three failed starts
  (`AppModel.swift`, `onGiveUp`), following the toggle and defaulting to on when no config can be read.
- **Change:** the same in the Linux app (`desktop/src/main.rs`). It already knows when the daemon is gone (the tray's
  daemon-down face) and already sends notifications (`notify_new_firmware`, through `notify-send`). When the daemon has
  been unreachable for 30 s, send one notification, then none until the daemon is back. Skip it when systemd reports the
  unit as `inactive` (`systemctl --user is-active vibebuddyd`), subject to the service-state decisions below. Follow
  the last known notify toggle, on when unknown, as the Mac does.
- **Assumptions to resolve before coding:** inspect the packaged unit and effective restart/start-limit settings;
  treat 30 s as a grace policy, not proof that systemd cannot restart again. Distinguish `inactive`, `failed`,
  `activating`, missing unit, unavailable user bus, command failure and timeout. `inactive` alone does not establish
  user intent. Record the suppression/fallback decision for each case before encoding it in tests.
- **String:** a Linux wording of the Mac's message, e.g. "The daemon stopped and didn't come back. Restart it from the
  tray icon or on the Advanced tab.", with its zh-Hans translation.
- **Test-infrastructure extension first:** extract a small timing/decision helper taking an explicit timestamp and
  service observation, similar to `LinkAlert` in `daemon/src/link_alert.rs`. Add a narrow headless test path through
  the same app-side policy and command adapter used by production. Configure and inspect the harness's
  `systemctl`/`notify-send` stubs there. Extending the tray snapshot probe alone cannot prove notification delivery;
  avoid a second test-only policy or a general notification framework.
- **Automated cases:** short outage/recovery; the grace boundary; one notification during a sustained outage;
  recovery followed by another outage; toggle off/on and unknown config; each service-state decision above;
  notification command timeout/failure. Verify both the decision and recorded notification arguments. Advance host
  test time directly, not the firmware clock; avoid repeated 30-second sleeps. Ensure the real app's stream failure
  and recovery observations reach the policy, rather than testing only hand-authored status snapshots.
- **Desktop acceptance:** app running, reproduce persistent daemon failure and an explicit service stop; verify
  actual notification visibility, suppression policy and rearming after recovery in the target user session.
  Record desktop/service versions. Headless passes with this check pending mean automated validation only.
- **Reflection before P1:** did tests reach the app's real daemon-down/notification path? Did service-state or
  startup behavior invalidate the assumptions? Keep a justified small seam; remove duplicate policy or unnecessary
  timing machinery. Update coverage and remaining acceptance in `docs/linux-host-testing.md`.
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
- **Test-infrastructure extension first:** add nested Sway-tree and X11-list fixtures, labeled synthetic or captured
  with source/version. Extend the existing stubs to respond to the relevant argument lists, including malformed/
  empty output, exit failures and bounded delays. Allow explicit per-test session variables in the child
  environment without inheriting the user's live compositor sockets. Keep the Hyprland scenarios as regressions.
- **Automated cases:** pure parser/selection tests for nearest ancestor, absent PID, multiple windows, invalid IDs
  and malformed output; backend selection with conflicting, absent or stale session evidence; missing executable,
  command error and timeout. From an actual simulated K2 release, verify the exact focus command, needs-input
  priority, completed-source retention and persistence after daemon restart. Retain local consumption of K0/K1,
  K2 hold and menu actions. Test each backend through the daemon, not just its parser or stub directly.
- **Desktop acceptance:** on Sway and X11, run an agent in a terminal and press the physical Adv's K2; verify the
  intended window gains focus, including after completion and daemon restart. Check the environment of the actual
  systemd-launched daemon; a successful shell command is insufficient. Recheck Hyprland when shared selection changes.
- **Reflection after each backend:** did fixture output match the real tool and version? Did stubs accept arguments
  the real tool rejects, or hide missing session variables? Keep both stdout and stderr in failure evidence. Add
  a minimal fixture/regression for a discovered mismatch before calling that backend validated.
- **Known limit, documented:** a terminal that runs all its windows in one process (for example gnome-terminal or a
  single-instance kitty) owns several windows under one pid, so K2 may pick the wrong one. Hyprland has the same limit
  today.
- **Not covered, documented:** GNOME and KDE Wayland window focus is outside this implementation; document the
  tmux-only fallback rather than claiming general Wayland coverage.

## P2: K2 into the Claude and Codex apps on Linux (investigate first)

- **Gap:** on the Mac, K2 opens the exact session or thread. On Linux the hook reports only process ancestors
  (`hook/src/surface.rs`) and never the desktop session ID (`hook/src/claude.rs`), so K2 at best focuses the app window,
  under Hyprland.
- **Step 1 (capability gate):** first verify official Linux app availability and record the installed app/version;
  the review's secondary reports are leads, not an established prerequisite. For each available app, inspect the
  hook environment and applicable navigation ID, including `CLAUDE_CODE_HOST_SESSION_ID` where relevant. Check
  `xdg-mime query default x-scheme-handler/claude` or `x-scheme-handler/codex`, and whether
  `xdg-open 'claude://code/continue?session=<id>'` or `xdg-open 'codex://threads/<id>'` actually selects that session.
  Verify the visible destination or consumer routing evidence, not just a zero exit code.
- **Step 2, only if step 1 holds:** the hook reports the desktop session ID on Linux too when the variable is set, and
  the daemon tries `xdg-open` with the same link before falling back to window focus. Tests: the link choice per
  platform in `source_opener.rs`. Limit implementation to the verified app/version combinations.
- **Tests if Step 2 proceeds:** add sanitized hook/environment fixtures and URL-choice/validation cases for
  missing/stale IDs, parent-thread navigation and invalid inputs. Drive K2 through the PTY and inspect `xdg-open`
  arguments, open failure and fallback to window focus. Stub success proves dispatch only; actual session selection
  remains a separate desktop acceptance result.
- **If Step 1 fails or cannot be verified:** record the app/version as unavailable, unverified or unsupported, with
  evidence and the window-focus fallback where supported. Defer that implementation; do not create fictitious
  capability fixtures.
- **Reflection:** did tests preserve the distinction between lifecycle identity, navigation identity and the host
  window? Record where simulation stops and what a real app demonstrated before updating the parity review.

## P3: say why there is no tray icon

- **Gap:** without a tray host the app opens its window after 10 s (`TrayHostCheck`) but doesn't say why. That happens
  on Fedora and upstream GNOME without the AppIndicator extension. Ubuntu preinstalls the extension; KDE and Omarchy have
  a tray.
- **Change:** document it in the README's Linux section (both languages). Only if users still miss it, add one line to
  the window when it opened for this reason; that line is Linux-only by nature (the Mac always has a menu bar).
- **Tests and acceptance:** the headless status/tray probe does not exercise tray discovery or window opening.
  Check a real session without a tray host and one with a host available; record the distribution/extension/session
  rather than treating all GNOME installations alike. If user evidence warrants UI later, first add the smallest
  seam to drive the real tray-detection/timeout decision; test absent host, delayed host and startup failure. Add
  shared zh-Hans wording. Do not build a tray-server emulator for the documentation-only change.
- **Reflection:** was documentation sufficient, or did observed behavior justify UI work? Record the evidence and
  keep speculative UI/harness expansion deferred.

## P4: documentation

- `README.md` and `README.zh-CN.md`, Linux section: K2 per desktop (Hyprland, then sway and X11 after P1; what GNOME
  and KDE Wayland do), the daemon-failure notification after P0, the tray note from P3, `wmctrl` as optional.
- `docs/app.md`: a short Linux section pointing to `docs/platform-parity.md` for the full table.
- `docs/platform-parity.md`: update each row as its item lands, separating implemented behavior, automated checks
  and desktop/device acceptance still pending. Retain source/version caveats for third-party apps.
- `docs/linux-host-testing.md`: record actual new scenarios, fixture provenance, runtime, known expected failures
  and remaining blind spots. Keep historical results distinct from fresh execution evidence.
- Keep the reflection log below and G4's result note current. Move a lesson to `LESSONS.md` only when an observed
  incident supports a reusable conclusion; avoid duplicating the protocol or introducing new domain vocabulary.

## Not planned

- A systemd `OnFailure=` alert (see P0).
- Self-updating on Linux: the package manager or `install.sh` stays the path; documented as by design.
- Terminal-tab focus on Linux, and Warp's deep link outside macOS: not requested, and the Mac's Automation permission has
  no Linux counterpart.
- Tests for the Mac app's UI: not a Linux gap, so out of this goal's scope.
- A line in the Agents tab explaining a failed K2: it would be a one-platform UI change for a logged condition; the docs
  cover it.
- Automatic expansion into Adv firmware identity, screenshot fixes, radio/Relay/ROM emulation or a general desktop
  testing framework. Record relevant follow-ups and keep the known gaps visible.

## Order and verification during execution

Order: T0 baseline and reflection; P0 with its tests and P4 updates; P0 reflection; P1 one backend at a time with tests,
documentation and reflection; P3 with the README pass; final P4 reconciliation. P2 capability research can proceed
when the relevant installed apps are available, and Step 2 remains conditional on its evidence.

For each implementation item, define observable behavior and add a focused regression before or alongside the fix.
Run focused checks while editing; run the complete affected suites at the item checkpoint. Repeat or broaden checks
only for a relevant change, failure or unresolved concern. A harness failure must not be hidden with longer sleeps,
silent retries, a removed assertion or a new expected-failure marker just to make the item pass.

The following commands are for later execution; listing them is not evidence they ran in this revision:

```sh
bash tools/test-linux-host.sh -k k2
bash tools/test-linux-host.sh
cargo test --workspace --locked
cargo test --locked -p vibebuddy-firmware-core --no-default-features \
  --features board-cardputer-adv --target-dir target/cardputer-adv
cargo clippy --workspace --all-targets --locked
python3 tools/check-localization.py
```

For each checkpoint, record revision, commands/results, expected failures and relevant artifact paths. Distinguish
code complete, automated checks passed, desktop/device acceptance pending, and deferred with reason. Full parity
acceptance requires the applicable real checks below; unavailable hardware/desktops do not prevent unrelated work,
but their checks remain pending.

| Check, on a real desktop | Hyprland | Sway | X11 | GNOME Wayland | KDE Wayland |
|---|---|---|---|---|---|
| K2 focuses intended terminal window | baseline recheck | P1 | P1 | window focus outside scope; tmux fallback | window focus outside scope; tmux fallback |
| K2 selects a Claude/Codex app session | P2 capability gate | P2 capability gate | P2 capability gate | P2 capability gate | P2 capability gate |
| Persistent daemon-failure notification and intentional-stop policy | P0 | P0 | P0 | P0 | P0 |
| Tray host available/absent and window fallback | record session | record session | depends on panel | record extension/session | record session |

## Reflection log to fill during execution

Keep entries short and evidence-based. At T0, after P0, after each P1 backend, after P2 capability checks and at the
P3/P4 closeout, reflect on whether the infrastructure tested the behavior that matters. Also add an entry whenever
real behavior contradicts a simulated pass or a test proves unreliable. These prompts do not imply a problem exists.

| Checkpoint | Status | Evidence / reflection entry |
|---|---|---|
| T0 current-revision baseline | Done 2026-10-17 | Entry below |
| P0 app notification path | Implemented; headless checks pass; desktop acceptance pending | Entry below |
| P1 Sway | Not started | Pending |
| P1 X11 and Hyprland regression | Not started | Pending |
| P2 capability gate / conditional implementation | Not started | Pending |
| P3/P4 coverage and documentation closeout | Not started | Pending |

**T0 entry, 2026-10-17, revision `0c1360a` (branch `claude/cardputer-adv-port`, clean tree).**

- **Observation:** `bash tools/test-linux-host.sh`: 30 tests, OK, one expected failure
  (`test_cardputer_screenshot_should_be_a_240_by_135_png`), 94 s. `cargo test --workspace --locked`: all pass; one test
  ignored, the native-USB ROM test that needs a device. Adv firmware-core (`--target-dir target/cardputer-adv`): 101 and
  47 pass. `cargo clippy --workspace --all-targets --locked`: exit 0 with four warnings (`firmware-rs/core/src/display.rs:322`,
  `daemon/src/activity.rs:313` and `:844`, `daemon/src/session_titles.rs:50`). `check-localization.py`: ok, 284 keys.
  Logs are in the session scratchpad, not the repo.
- **Expected failure, checked:** run unmarked, it fails at its first assertion with HTTP 502
  `incomplete frame: 32400 pixels`. `daemon/src/screenshot.rs:12–13` and `:125` hard-code 320×240, so the daemon
  rejects a 240×135 frame. This is a product gap, not a harness defect. The plan's wording ("dimension check") is
  imprecise and should say so when the gap is next touched.
- **Isolation and cleanup:** the run was in a Wayland desktop session (`DISPLAY`, `WAYLAND_DISPLAY` set, no `SWAYSOCK`).
  The daemon gets only `PATH` and explicit XDG paths, and no `vibebuddyd` or simulator process was left behind.
  Control sockets are removed on close. Scenario artifacts in `target/host-tests` are kept after passing runs, with no
  pruning: 140 directories, 9.8 MB after this baseline. Not a failure, but it grows without bound locally.
- **Coverage assessment:** the Adv firmware is simulated, over the PTY with bridge pacing, so native USB is untested
  (already recorded in `docs/linux-host-testing.md`). The desktop harness covers only the headless status and menu
  path. The notification decision and command calls (`desktop/src/main.rs:343`, `:1762`) are reached only from iced's
  `update()`, which the probe cannot run.
- **Seams for P0 and P1, found during T0 (not built yet):**
  - P0: the smallest seam is a pure outage-policy struct shaped like `LinkAlert`, compiled into
    `desktop/examples/host_status_probe.rs` through `#[path]`, as `status.rs` already is. It takes explicit `Instant`s
    and Down/Up events, and calls `notify-send` through the same adapter production uses. The `systemctl` stub gives one
    reply per program, so the `is-active` replies work only if no other `systemctl` call in the same test needs a different reply.
  - P1: stub replies are keyed by program name only (`tools/host_test_support.py`, `command_reply`). `swaymsg -t get_tree`
    and `swaymsg [con_id=N] focus` cannot return different output, and stubs have no stderr, no delay and no per-argument
    replies. The daemon's environment has no `DISPLAY`, `WAYLAND_DISPLAY`, `SWAYSOCK` or `XDG_SESSION_TYPE`, so session
    selection cannot be exercised until per-test session variables are allowed.
- **Decision:** T0 is the baseline; no product or harness code changed. Next is P0, after the toggle decision in the
  plan's open question (the Mac's give-up path does not follow the toggle, and the Linux app clears its status on
  daemon loss).

**P0 entry, 2026-10-17, on `0c1360a` with the P0 changes uncommitted.** The user said "go ahead" without answering the toggle
question, so I took the recommended reading: Linux keeps the last-known toggle, on when none was read. The Mac is unchanged, and
the difference is recorded for a maintainer decision (`docs/platform-parity.md`, gap 2).

- **Observation:** full host runner 39 scenarios OK (30 existing, 9 new `DaemonOutageTests`), one expected failure, 155 s.
  Workspace 502 passed; Adv core 101 and 47; clippy exit 0 with the 4 pre-existing warnings; localization 285 keys.
  `desktop` unit tests: 13 new outage policy tests. Mutation check: an `inactive` suppression removed and the toggle
  ignored in one run; each failed exactly its own scenario (`a_unit_stopped_on_purpose_stays_silent`,
  `the_notify_toggle_silences_an_outage_after_the_daemon_is_gone`). Logs: scratchpad `p0/`.
- **Coverage assessment:** production path covered: the app's real status stream (`client::status_updates_at`), the
  outage state machine (`desktop/src/outage.rs`), and its `systemctl` and `notify-send` adapters, all driven by
  `daemon_outage_probe` against the real daemon. Not covered: the iced wiring in `desktop/src/main.rs` (`update`,
  the `Task` mapping, the notification from the real app); it compiles and is checked only by reading. Stubs are
  fire-and-forget on the notify side, so a missing `notify-send` binary is not exercised. The probe shortens the grace
  to 4 s, because the stream retries every 2 s; the 30 s boundary is checked with explicit instants.
- **Decisions taken while building the seam:**
  - One policy, shared by app and probe: `Outage::handle` returns effects (`Wait`, `CheckService`, `Notify`) and each
    caller runs them. No second policy in the test.
  - Unit states: `active`, `activating`, `failed`, `Unknown` (no bus, missing binary, timeout after 2 s) all notify.
    `inactive` and `deactivating` stay silent: the user's stop. `is-active` prints `inactive` for a missing unit too, so
    a missing unit is silent, not distinguished (this was an open question in the plan; it is now a recorded limit).
  - The last-known toggle survives the daemon going away, because `Update::Down` clears `status` (`desktop/src/main.rs`).
  - Only the first `Down` of an outage starts the timer; the stream repeats `Down` every 2 s while the daemon is gone.
  - `client.rs` gained `status_updates_at(base)`, the test seam for the probe's port. Production calls it with the
    fixed `127.0.0.1:7331`.
  - The harness config turns notifications off for the other tests; the outage tests turn the toggle on in `setUp`, and
    count only `VibeBuddy` titles, since the daemon's own box alert also uses `notify-send`.
  - `notify-send` is spawned and not awaited, like the firmware notification, so the adapter has no timeout.
- **Follow-through:** docs updated: `README.md` and `README.zh-CN.md` (Linux section, one sentence each),
  `docs/platform-parity.md` (notification row and gap 2), `docs/linux-host-testing.md` (scenario row, stubs, acceptance,
  validation). zh-Hans string added. Remaining acceptance: on a real desktop, a persistent daemon failure and an explicit
  `systemctl --user stop` should give the notifications and silences the tests predict, and the tray restart should
  clear it. Not run: a real notification server, a real systemd unit, the Mac app. P1 is next, one backend at a time, and
  the outage seam is reused as it stands.

For an entry, record:

- **Observation:** revision, expected behavior, actual result and the log, fixture or desktop/device evidence.
- **Coverage assessment:** which production path ran; which layer was simulated, stubbed or untested; whether the
  result exposed a product bug, a harness defect, an environment limitation or an unresolved assumption.
- **Decision:** reuse as is, tighten a fixture/assertion, add a small seam, simplify/remove an unreliable test, or
  leave explicit real acceptance/follow-up. Explain why the change improves confidence at reasonable cost.
- **Follow-through:** regression added or planned, relevant plan/coverage-doc change and remaining acceptance status.

When a real check contradicts a simulated pass, preserve the evidence, identify the missing contract or assumption,
and add a focused regression where possible. Update that item's scope or acceptance before claiming completion;
do not turn every discovery into a general emulator or unrelated product change.
