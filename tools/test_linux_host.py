"""Run through tools/test-linux-host.sh; ordinary tool discovery skips this integration suite."""

from __future__ import annotations

import base64
from concurrent.futures import ThreadPoolExecutor
import json
import os
import re
import struct
import subprocess
import sys
import time
import unittest
from pathlib import Path

from host_test_support import HostHarness, REPO, wait_for
from make_voice_pack import build as build_voice_pack
from character_pack import build as build_character_pack

ENABLED = sys.platform.startswith("linux") and os.environ.get("VIBEBUDDY_HOST_TESTS") == "1"
FIXTURES = Path(__file__).with_name("fixtures") / "cardputer-adv"


@unittest.skipUnless(ENABLED, "run tools/test-linux-host.sh on Linux")
class HostTestCase(unittest.TestCase):
    firmware = True

    def setUp(self):
        self.host = HostHarness(self._testMethodName)
        self.addCleanup(self.host.close)
        self.host.start(firmware=self.firmware)

    def post(self, path, body=None, expected=202):
        code, response = self.host.request(path, body, method="POST")
        self.assertEqual(code, expected, response)
        return response

    def event(self, event_name, **fields):
        return self.post("/v1/events", {"version": 1, "event": event_name, **fields})

    def hook(self, name, session="test-session", pids=None):
        return self.post("/v1/claude-hooks", {
            "session_id": session, "prompt_id": "test-turn", "hook_event_name": name,
            "surface": "window", "host_pids": pids or [4242, 4000],
        })


class LinuxHostTests(HostTestCase):
    def test_boot_identity_pairing_and_isolated_config(self):
        snapshot = self.host.status()
        device = snapshot["device"]
        self.assertFalse(device["foreign_firmware"])
        # PTYs have no USB metadata; the existing explicit-port path conservatively enables pacing.
        self.assertTrue(device["bridge"])
        self.assertFalse(device["network"])
        self.assertEqual(device["firmware_build"], "simulator")
        version = re.search(r'^version = "([^"]+)"', (REPO / "Cargo.toml").read_text(), re.MULTILINE)[1]
        self.assertEqual(device["firmware_version"], version)
        self.assertEqual(device["voice"], "builtin")
        # The current Adv device firmware does not emit BOARD. Do not invent one in the simulator.
        self.assertIsNone(device["unsupported_board"])
        self.assertEqual(len(device["box_key"]), 43)
        self.assertTrue(any(computer["key"] == device["computer_key"] for computer in device["paired_computers"]))
        self.assertFalse(snapshot["config"]["notify_link"])
        self.assertFalse(snapshot["config"]["check_updates"])
        self.assertFalse(snapshot["config"]["relay"])

    def test_explicit_board_diagnostic_reaches_host(self):
        self.host.emit("BOARD cardputer-adv\n")
        self.host.wait_device(lambda device: device["unsupported_board"] == "cardputer-adv", "board diagnostic")

    def test_volume_identify_and_device_menu_updates(self):
        self.post("/v1/device/identify")
        self.host.wait_line("IDENTIFY\n")
        self.post("/v1/device/volume", {"level": 40, "preview": True})
        self.host.wait_device(lambda device: device["volume"] == 40, "volume acknowledgment")
        self.host.wait_line("AUDIO QUEUED DONE")
        self.post("/v1/device/volume", {"level": 0}, expected=400)
        self.host.emit("VOLUME ERROR\r\nVOLUME 75\r\n")
        self.host.wait_device(lambda device: device["volume"] == 75, "unsolicited volume")

    def test_http_to_firmware_all_activity_states(self):
        for event, title, output in (
            ("task.start", "WORK", "DISPLAY STATE WORKING"),
            ("agent.input_required", "ASK", "DISPLAY STATE INPUT REQUIRED"),
            ("task.error", "FAIL", "DISPLAY STATE FAILED"),
            ("task.done", "DONE", "DISPLAY STATE DONE"),
        ):
            with self.subTest(event=event):
                self.event(event, title=title)
                self.host.wait_line(output)
        self.host.wait_line("AUDIO QUEUED INPUT_REQUIRED")
        self.host.wait_line("AUDIO QUEUED FAILED")

    def test_k2_routes_active_completed_and_persisted_source(self):
        self.host.command_reply("hyprctl", clients=[{"pid": 4242, "address": "0xabc"}])
        self.hook("UserPromptSubmit")
        self.host.press("K2")
        wait_for(lambda: self.host.commands("hyprctl"), "K2 opens window")
        wait_for(lambda: any("0xabc" in " ".join(command["args"]) for command in self.host.commands("hyprctl")),
                 "correct source focused")
        self.hook("Stop")
        before = len(self.host.commands("hyprctl"))
        self.host.press("K2")
        wait_for(lambda: len(self.host.commands("hyprctl")) > before, "completed source still opens")
        self.host.stop_daemon()
        self.host.start_daemon()
        self.host.wait_device(lambda device: device["paired"], "daemon restart handshake", timeout=12)
        before = len(self.host.commands("hyprctl"))
        self.host.press("K2")
        wait_for(lambda: len(self.host.commands("hyprctl")) > before, "source survives daemon restart")

    def test_k2_prefers_needs_input_over_other_work(self):
        self.host.command_reply("hyprctl", clients=[
            {"pid": 4242, "address": "0xaaa"}, {"pid": 4343, "address": "0xbbb"},
        ])
        self.hook("UserPromptSubmit", session="needs-input")
        self.hook("PermissionRequest", session="needs-input")
        self.hook("UserPromptSubmit", session="other-work", pids=[4343])
        self.host.press("K2")
        wait_for(lambda: any("0xaaa" in " ".join(command["args"]) for command in self.host.commands("hyprctl")),
                 "K2 selects needs input")
        self.assertFalse(any("0xbbb" in " ".join(command["args"]) for command in self.host.commands("hyprctl")))

    def test_k2_uses_nearest_ancestor_and_legacy_hyprland_fallback(self):
        self.host.command_reply("hyprctl", clients=[
            {"pid": 4000, "address": "0xaaa"}, {"pid": 4242, "address": "0xbbb"},
        ], lua_exit=1, lua_stdout="unknown Lua dispatch")
        self.hook("UserPromptSubmit")
        self.host.press("K2")
        wait_for(lambda: any(command["args"] == ["dispatch", "focuswindow", "address:0xbbb"]
                             for command in self.host.commands("hyprctl")), "legacy focus of nearest ancestor")

    def test_k2_failed_desktop_command_recovers_on_next_press(self):
        self.host.command_reply("hyprctl", exit=1, stdout="compositor unavailable")
        self.hook("UserPromptSubmit")
        self.host.press("K2")
        wait_for(lambda: "compositor unavailable" in (self.host.root / "daemon.log").read_text(), "focus failure logged")
        self.host.command_reply("hyprctl", clients=[{"pid": 4242, "address": "0xabc"}])
        self.host.press("K2")
        wait_for(lambda: any("0xabc" in " ".join(command["args"]) for command in self.host.commands("hyprctl")),
                 "next K2 recovers")

    def test_long_press_and_menu_consume_k2_locally(self):
        self.hook("UserPromptSubmit")
        self.host.press("K2", long=True)
        self.host.wait_line("MUTE ON")
        self.host.press("K1", long=True)
        self.host.wait_line("MENU OPEN")
        self.host.press("K2")
        self.host.emit("VOLUME 55\n")
        self.host.wait_device(lambda device: device["volume"] == 55, "device output drained")
        self.assertEqual(self.host.commands("hyprctl"), [])
        self.assertNotIn('"event":"button"', self.host.lines())

    def test_k0_k1_pomodoro_and_clock_are_local(self):
        self.host.press("K1")
        self.host.wait_device(lambda device: device["mode"] == "pomodoro", "K1 mode")
        self.host.press("K0")
        self.host.wait_line("POMODORO FOCUS START")
        self.host.press("K0")
        self.host.wait_line("POMODORO PAUSED")
        self.host.press("K0")
        self.host.wait_line("POMODORO RESUMED")
        self.host.control("advance", ms=25 * 60 * 1000 + 100)
        self.host.wait_line("POMODORO FOCUS END")
        self.assertEqual(self.host.commands("hyprctl"), [])

    def test_unplug_replug_relearns_state_and_keeps_device_storage(self):
        self.post("/v1/device/volume", {"level": 45})
        old = self.host.wait_device(lambda device: device["volume"] == 45, "volume stored")["device"]["box_key"]
        self.host.disconnect()
        self.host.wait_device(lambda device: not device["connected"], "unplug")
        self.host.connect()
        self.host.wait_device(lambda device: device["connected"], "replug")
        self.host.control("reboot")
        snapshot = self.host.wait_device(lambda device: device["paired"] and device["volume"] == 45, "reboot restores NVS")
        self.assertEqual(snapshot["device"]["box_key"], old)

    def test_pair_list_and_unpair_round_trip(self):
        other = base64.urlsafe_b64encode(bytes(range(32))).decode().rstrip("=")
        self.event("device.pair", key=other, name="Simulated second computer")
        self.event("device.pairs")
        self.host.wait_device(lambda device: any(pair["key"] == other for pair in device["paired_computers"]), "second pair")
        self.post("/v1/device/unpair", {"key": other})
        self.host.wait_device(lambda device: all(pair["key"] != other for pair in device["paired_computers"]), "unpaired")
        own = self.host.status()["device"]["computer_key"]
        self.post("/v1/device/unpair", {"key": own}, expected=409)

    def test_wifi_device_layer_signals_reach_host(self):
        fixture = json.loads((FIXTURES / "wifi.json").read_text())
        self.post("/v1/device/wifi", {"ssid": "Test network", "password": "test-only"})
        self.host.wait_line("WIFI NETWORK Test network")
        self.post("/v1/device/wifi/scan")
        self.host.emit(fixture["scan"])
        snapshot = self.host.wait_device(lambda device: len(device["wifi_seen"]) == 2, "scan results")
        self.assertEqual([item["ssid"] for item in snapshot["device"]["wifi_seen"]], ["Office test", "Test network"])
        self.host.emit(fixture["failed"])
        self.host.wait_device(lambda device: bool(device["wifi_error"]), "Wi-Fi failure")
        self.host.emit(fixture["joined"])
        self.host.wait_device(lambda device: device["wifi_address"] == "192.0.2.10" and not device["wifi_error"], "Wi-Fi recovered")
        self.host.emit("WIFI DISCONNECTED\n")
        self.host.wait_device(lambda device: device["wifi_address"] is None, "Wi-Fi disconnect")
        self.post("/v1/device/wifi/forget")
        self.host.wait_device(lambda device: device["wifi_network"] is None, "Wi-Fi forgotten")

    @staticmethod
    def pack():
        return build_voice_pack("test-voice", [bytes([index, 0, index, 0]) * 64 for index in range(5)])

    def test_voice_pack_acknowledgments_and_persistence(self):
        self.post("/v1/device/voice-pack", self.pack())
        snapshot = self.host.wait_operation("done")
        self.assertEqual(snapshot["config"]["voice"], "test-voice")
        self.host.wait_device(lambda device: device["voice"] == "test-voice", "written voice reported")
        self.host.control("reboot")
        self.host.wait_device(lambda device: device["paired"] and device["voice"] == "test-voice", "voice survives reset")

    def test_character_pack_uses_the_same_write_protocol(self):
        pack = build_character_pack("test-character", {"done": [[0] * 128]})
        self.post("/v1/device/voice-pack", pack)
        snapshot = self.host.wait_operation("done")
        self.assertEqual(snapshot["config"]["voice"], "test-character")
        self.host.wait_device(lambda device: device["voice"] == "test-character", "Character installed")

    def test_pack_larger_than_adv_partition_is_refused(self):
        self.post("/v1/device/voice-pack", self.pack() + bytes(0x1C0000))
        snapshot = self.host.wait_operation("failed")
        self.assertIn("device refused", snapshot["operation"]["message"])
        self.assertNotIn('"event":"voice.chunk"', self.host.lines("host"))

    def test_device_crc_failure_becomes_failed_operation(self):
        pack = bytearray(self.pack())
        pack[-1] ^= 1
        self.post("/v1/device/voice-pack", bytes(pack))
        snapshot = self.host.wait_operation("failed")
        self.assertIn("device refused", snapshot["operation"]["message"])
        self.assertIsNone(snapshot["config"]["voice"])
        self.post("/v1/device/voice-pack", self.pack())
        self.host.wait_operation("done")

    def test_missing_or_wrong_ack_stops_next_chunk_and_serializes_operations(self):
        self.host.filter(drop=('{"version":1,"event":"voice.ack"',))
        self.post("/v1/device/voice-pack", self.pack())
        self.host.wait_line('"event":"voice.ack"')
        self.host.emit('{"version":1,"event":"voice.ack","seq":999}\nVOLUME 55\n')
        self.host.wait_device(lambda device: device["volume"] == 55, "wrong ack processed")
        self.post("/v1/device/voice-pack", self.pack(), expected=409)
        self.assertNotIn('"seq":1', self.host.lines("host"))
        self.host.emit('{"version":1,"event":"voice.error","message":"injected write failure"}\n')
        snapshot = self.host.wait_operation("failed")
        self.assertIn("injected write failure", snapshot["operation"]["message"])

    def test_lost_receipt_times_out_and_allows_retry(self):
        self.host.filter(drop=('{"version":1,"event":"voice.ready"',))
        self.post("/v1/device/voice-pack", self.pack())
        snapshot = self.host.wait_operation("failed", timeout=24)
        self.assertIn("timed out waiting for voice.ready", snapshot["operation"]["message"])
        self.host.filter()
        self.post("/v1/device/voice-pack", self.pack())
        self.host.wait_operation("done")

    def test_delayed_duplicate_receipt_resumes_the_write(self):
        self.host.filter(drop=('{"version":1,"event":"voice.ack"',))
        self.post("/v1/device/voice-pack", self.pack())
        self.host.wait_line('"event":"voice.ack"')
        self.assertNotIn('"seq":1', self.host.lines("host"))
        self.host.filter()
        receipt = '{"version":1,"event":"voice.ack","seq":0}\n'
        self.host.emit(receipt + receipt)
        snapshot = self.host.wait_operation("done")
        self.assertEqual(snapshot["config"]["voice"], "test-voice")

    def test_unplug_aborts_an_in_progress_write(self):
        self.host.filter(drop=('{"version":1,"event":"voice.ready"',))
        self.post("/v1/device/voice-pack", self.pack())
        self.host.wait_line('"event":"voice.ready"')
        self.host.disconnect()
        snapshot = self.host.wait_operation("failed")
        self.assertEqual(snapshot["operation"]["message"], "link lost")

    def test_sse_delivers_device_changes_and_config(self):
        request = self.host.opener.open(self.host.base + "/v1/status/stream", timeout=5)
        with request:
            self.post("/v1/device/volume", {"level": 35})
            for _ in range(100):
                line = request.readline()
                if line.startswith(b"data:"):
                    snapshot = json.loads(line[5:])
                    if snapshot["device"]["volume"] == 35:
                        self.assertFalse(snapshot["config"]["relay"])
                        break
            else:
                self.fail("SSE never delivered the volume acknowledgment")

    def test_linux_app_status_and_tray_model_consumes_live_snapshots(self):
        snapshots = [self.host.status()]
        self.host.press("K1")
        snapshots.append(self.host.wait_device(lambda device: device["mode"] == "pomodoro", "mode for Linux app"))
        self.host.disconnect()
        snapshots.append(self.host.wait_device(lambda device: not device["connected"], "offline for Linux app"))
        snapshots.append(None)  # What the app derives when the daemon stops answering.
        probe = os.environ.get("VIBEBUDDY_TEST_DESKTOP_PROBE", str(REPO / "target/debug/examples/host_status_probe"))
        result = subprocess.run([probe], input="".join(json.dumps(snapshot) + "\n" for snapshot in snapshots),
                                text=True, capture_output=True, env=self.host.env, timeout=8, check=True)
        menus = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual([menu["icon"] for menu in menus], ["online", "online", "offline", "daemon_down"])
        self.assertEqual(menus[1]["mode_line"], "Mode: Pomodoro")
        self.assertIn("click to restart", menus[3]["device_line"])

    def test_malformed_fragmented_and_coalesced_device_output(self):
        self.host.filter(fragment=3)
        self.host.emit(b'not-json\n{"version":99,"event":"button","button":"K2","action":"press"}\n'
                       b'{broken json}\nVOLUME ERROR\nVOLUME 2')
        # Separate writes exercise the daemon's partial-line buffer across reads.
        self.host.emit(b'5\r\nMODE POMODORO\r\n')
        self.host.wait_device(lambda device: device["volume"] == 25 and device["mode"] == "pomodoro", "fragmented lines")
        self.assertEqual(self.host.commands("hyprctl"), [])
        self.host.emit(b'x' * 9000 + b'\nVOLUME 60\n')
        self.host.wait_device(lambda device: device["volume"] == 60, "oversized diagnostic recovery")

    def test_cardputer_screenshot_rejection_is_explicit(self):
        code, response = self.host.request("/v1/device/screenshot", method="POST")
        # Characterize the existing host limitation without fixing production in the infrastructure pass.
        self.assertEqual(code, 502, response)
        self.assertIn(b"incomplete frame: 32400 pixels", response)
        self.host.wait_line("SHOT BEGIN 240x135")

    def test_synthetic_box_screenshot_still_encodes_as_png(self):
        # Synthetic BOX-size compatibility data, explicitly separate from the real Adv frame.
        self.host.filter(drop=("SHOT",))
        with ThreadPoolExecutor(max_workers=1) as executor:
            capture = executor.submit(self.host.request, "/v1/device/screenshot", method="POST")
            self.host.wait_line("SHOT END")  # The actual Adv dump has been completely filtered out.
            self.host.emit("SHOT BEGIN 320x240 BACKLIGHT ON\nSHOT f800:76800\nSHOT END\n")
            code, png = capture.result(timeout=8)
        self.assertEqual(code, 200, png)
        self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")
        self.assertEqual(struct.unpack(">II", png[16:24]), (320, 240))

    def test_malformed_screenshot_becomes_an_http_error(self):
        self.host.filter(drop=("SHOT",))
        with ThreadPoolExecutor(max_workers=1) as executor:
            capture = executor.submit(self.host.request, "/v1/device/screenshot", method="POST")
            self.host.wait_line("SHOT END")
            self.host.emit("SHOT BEGIN 240x135 BACKLIGHT ON\nSHOT invalid:12\nSHOT END\n")
            code, response = capture.result(timeout=8)
        self.assertEqual(code, 502, response)
        self.assertIn(b"bad color", response)

    @unittest.expectedFailure
    def test_cardputer_screenshot_should_be_a_240_by_135_png(self):
        code, png = self.host.request("/v1/device/screenshot", method="POST")
        # Keep unexpected transport/setup errors outside the expected-failure assertion.
        self.host.wait_line("SHOT END")
        self.assertEqual(code, 200, png)
        self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")
        self.assertEqual(struct.unpack(">II", png[16:24]), (240, 135))


@unittest.skipUnless(ENABLED, "run tools/test-linux-host.sh on Linux")
class ForeignFirmwareTests(HostTestCase):
    firmware = False

    def test_recorded_adv_boot_is_recognized(self):
        # From device-specs.md; replace the redacted device key with a synthetic 32-byte key.
        self.host.emit((FIXTURES / "boot.txt").read_bytes())
        snapshot = self.host.wait_device(lambda device: device["firmware_build"] is not None
                                         and device["volume"] == 65, "recorded Adv boot")
        self.assertFalse(snapshot["device"]["foreign_firmware"])
        self.assertEqual(snapshot["device"]["firmware_version"], "0.5.0")
        self.assertIsNone(snapshot["device"]["unsupported_board"])
        self.assertIn("MEMORY INTERNAL 38848", self.host.lines("delivered"))

    def test_foreign_firmware_receives_no_probe_or_heartbeat(self):
        self.host.emit("I (1234) factory: waiting for keyboard\r\n")
        self.host.wait_device(lambda device: device["foreign_firmware"], "foreign firmware detected")
        # Longer than one heartbeat and the listen-first interval: no hello, no JSON key presses.
        time.sleep(6.2)
        self.assertEqual(self.host.lines("host"), "")
        self.host.emit("ESP-ROM:esp32s3-20210327\r\nDISPLAY READY BUILD recovered\nVOLUME 65\n")
        self.host.wait_device(lambda device: not device["foreign_firmware"] and device["firmware_build"] == "recovered",
                              "reset reclassifies firmware")
        self.host.wait_line('"event":"device.heartbeat"', direction="host")


if __name__ == "__main__":
    unittest.main()
