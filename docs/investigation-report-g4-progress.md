# Investigation Report: G4 Progress and Remaining Gaps

**Date:** 2026-10-17
**Scope:** G4's goal, parity review and implementation plan; implementation through `868b07c`, particularly commits `0c1360a`, `9ff71af` and `8556bb3`.

## 1. Objective

Check whether the recent computer-side parity work matches its plan and identify incomplete behavior, inconsistent claims and missing validation.

The original review and planning deliverables exist. P0 and P1 have implementation and headless coverage; P3 deliberately delivers documentation; P4 updates the surrounding docs. That is meaningful progress, but the implementation still has the defects below and its desktop acceptance is incomplete.

## 2. Methodology

Read the goal, plan, parity matrix, architecture, domain vocabulary and relevant lessons. Trace notification events from the status stream through the outage policy and iced effects; compare restart actions on Mac and Linux; trace compositor selection, helper execution, packaging and host-test assertions.

This is a static review. No tests, builds, installs or downloads were performed. Existing test counts are historical claims in the project documents, not independently reproduced results. Only this report was added; application code is unchanged.

## 3. Key Findings

### 3.1 [P1] The outage notification directs users to a disabled recovery action

- **Evidence:** `desktop/src/outage.rs:127` says, “Restart it from the tray icon or on the Advanced tab.” `desktop/src/main.rs:351` clears `status` on daemon loss, while `:1591` enables Advanced's restart button only when `self.status.is_some()`.
- **Trace:** Advanced dispatches `RestartDaemon` (`main.rs:623`), which calls the daemon's own HTTP endpoint (`desktop/src/client.rs:153–154`). The tray instead dispatches `StartDaemon` (`main.rs:1950`), which executes `systemctl --user restart vibebuddyd` (`:628–630`).
- **Impact:** Following the new notification to Advanced cannot recover a stopped daemon. Without a tray host, neither advertised route is available. This also contradicts the parity matrix's broadly “same” Advanced restart row (`docs/platform-parity.md:46`): the Mac button remains enabled and calls its supervisor (`app/Sources/VibeBuddy/SettingsView.swift:800`, `AppModel.swift:147–148`).
- **Follow-up:** Make Advanced use the systemd recovery path while down. Verify recovery from the actual window with no tray host, including command failure.

### 3.2 [P2] Suppression for an inactive service never gets reconsidered

- **Evidence:** `desktop/src/outage.rs:87–94` creates one timer on the first `Down`; every later `Down` returns `None`. At `:95–98`, an inactive service suppresses notification without scheduling another observation. Only `Status` resets the outage (`:81–84`).
- **Trigger:** Stop the service long enough for the inactive check to suppress the alert. Later restart it, but have every attempt fail before returning a status snapshot. The policy continues receiving `Down`, schedules no service check, and never discovers the new failure.
- **Impact:** A genuine startup failure can remain silent indefinitely after an intentional stop. The existing suppression test ends at the first inactive result (`tools/test_linux_host.py:473–479`); the recovery test requires a successful status between outages (`:443–452`). Neither covers this transition.
- **Follow-up:** Reconsider the service after suppression, through bounded observations or a service-state signal. Add the inactive → failed transition without an intervening status snapshot as a regression.

### 3.3 [P2] An unresponsive daemon can bypass the outage timer entirely

- **Evidence:** `desktop/src/client.rs:29–43` creates a default HTTP client, waits for response headers and then awaits `body.next()` without a deadline. `Update::Down` is sent only after those operations return. The new policy starts its grace period only on `Down` (`desktop/src/outage.rs:87–92`).
- **Dependency check:** The locked reqwest version is 0.13.5 (`Cargo.lock:3689–3690`). Its locally cached source, `reqwest-0.13.5/src/async_impl/client.rs:300,314–315`, defaults connect, read and total timeouts to `None`.
- **Trigger:** A daemon accepts the connection and then stalls, or stops producing bytes on an established stream without closing the socket.
- **Impact:** The app can retain stale “running” status and never begin the promised outage grace period. This originates in the existing client, but limits the new failure detector. The host tests stop the daemon process; they do not cover an open, stalled connection (`tools/test_linux_host.py:420–503`).
- **Follow-up:** Bound header acquisition and stream inactivity. Use incoming SSE traffic, including keepalives, as liveness evidence; the daemon already configures keepalives (`daemon/src/main.rs:540`). Add a stalled-stream case. Avoid a fixed total lifetime for a healthy long-lived stream.

### 3.4 [P2, conditional deployment risk] K2 depends on session variables reaching an already-started systemd daemon

- **Evidence:** `daemon/src/source_opener.rs:512–516` selects the backend from the daemon's own environment. `daemon/src/window_focus.rs:25–35` defaults to Hyprland without the required Sway/X11 evidence. The unit starts under `default.target` (`packaging/linux/vibebuddyd.service:11`), and the installer enables it directly (`packaging/linux/install.sh:44–45`). Neither supplies a session-environment refresh mechanism.
- **Coverage boundary:** The Sway tests inject `SWAYSOCK` directly and create a regular file at that path (`tools/test_linux_host.py:517–520`); X11 tests inject `XDG_SESSION_TYPE` and `DISPLAY` (`:620–621`). These verify backend dispatch, not environment delivery at login. The plan itself records the unresolved production-environment check (`.agent_context/plans/G4-linux-parity-plan.md:342–345`).
- **Impact:** If the service starts before the desktop imports its environment, K2 can keep choosing Hyprland throughout a Sway/X11 session. Whether a supported desktop actually hits this is unverified; this review does not claim universal breakage.
- **Follow-up:** Accept P1 using the packaged service after a fresh login. If necessary, add session startup integration that refreshes the daemon's environment, and document required setup.

### 3.5 [P3] The current progress summary still describes an earlier repository state

- **Evidence:** `.agent_context/goals/G4.md:11–12` still summarizes Hyprland-only focus and no Linux daemon-failure alert. Its latest implementation entry says “uncommitted, on `0c1360a`” (`:36`), although the feature commits are now `9ff71af` and `8556bb3`, followed by the documentation commit `868b07c`.
- **Further ambiguity:** The plan opens with “planning only” and no executed checks (`.agent_context/plans/G4-linux-parity-plan.md:3–4`), but later contains implementation results. Its P0 description says the Mac follows the toggle (`:65,70`), while its execution entry correctly records the opposite (`:273–275`).
- **Impact:** Readers must reconstruct chronology to distinguish historical assumptions from current status. These are documentation issues, not evidence that the recorded tests failed.
- **Follow-up:** Add a current-status summary with committed revisions, label older descriptions as historical, and correct the superseded Mac-toggle premise.

## 4. Remaining Acceptance and Parity Work

| Item | Current evidence | Still needed |
|---|---|---|
| P0 notification | Shared policy, real status stream and command stubs; iced wiring only inspected (`plan:282–287`) | Actual notification delivery, intentional stop, failed restart and recovery through the real UI/systemd unit |
| P1 Sway/X11 | Synthetic listings and helper stubs (`docs/linux-host-testing.md:194–202`) | Real helper output, visible destination, fresh-login service environment, completion and restart behavior |
| P2 desktop session links | Linux hook still always reports `Surface::Window` (`hook/src/surface.rs:59–62`); capability gate not run (`plan:369–374`) | Installed app/version, navigation identity, URL registration and visible session selection |
| P3 missing tray | README explanation; Ubuntu host-present and isolated Xvfb host-absent observations (`plan:350–366`) | Fedora/upstream GNOME without the extension and KDE acceptance |
| Mac notification toggle | Clearing status before `onGiveUp` makes its check default to on (`app/Sources/VibeBuddy/AppModel.swift:43–50`) | Resolve the documented behavior difference; this is a pre-existing Mac defect, not a new Linux regression |
| Adv screenshot and board identity | Explicitly recorded gaps (`docs/linux-host-testing.md:122–129`) | Separate device work; neither should be represented as solved by the host harness |

## 5. Gaps and Uncertainties

- The three notification/recovery findings follow directly from control flow, but were not reproduced in a running app during this review.
- Sway/X11 deployment behavior remains conditional on the target desktop's service environment. Headless passes cannot resolve it.
- P0's plan requests notification command failure coverage (`plan:83–84`), while the implementation log acknowledges the missing-helper case is untested (`:286`).
- The test log's 55 host scenarios and 522 workspace tests are recorded implementation evidence (`docs/linux-host-testing.md:194–198`), not new results from this review.

## 6. Conclusions

G4 has delivered its review and plan and implemented much of the first parity wave. Before treating the implementation as accepted, address Advanced recovery, suppressed-outage reconsideration and stalled-stream detection, then complete desktop/service acceptance. Keep P2 and the Mac-toggle decision explicitly open and refresh the goal's current-state summary.

## 7. Implementation Follow-up

The subsequent fix restores Advanced recovery through the same bounded systemd adapter as the tray, rechecks inactive
services after another grace period, and adds header/inactivity deadlines to the Linux status stream. The Mac stream
uses the same 45 s inactivity limit and its failure notification retains the last reported toggle. The goal and parity
summaries are updated. The findings above describe the reviewed `868b07c` state; current validation is recorded in
`linux-host-testing.md`. Real desktop acceptance, the systemd session-environment check and P2 navigation remain open.
