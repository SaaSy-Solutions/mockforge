"""Fail-closed checks for the legacy Fly Machine cutover controller."""

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "ops" / "fly_legacy_cutover.py"
SPEC = importlib.util.spec_from_file_location("fly_legacy_cutover", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
cutover = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(cutover)


def rows():
    result = {}
    for index, app in enumerate(cutover.APPS):
        count = 2 if index == 0 else 1
        result[app] = [
            {
                "id": f"machine-{index:02d}-{number}",
                "name": f"{app}-{number}",
                "region": "iad",
                "state": "started",
                "image_ref": "registry.fly.io/example:stable",
                "config": {"services": [{"autostart": True}]},
            }
            for number in range(count)
        ]
    return result


class FlyLegacyCutoverTests(unittest.TestCase):
    def test_inventory_fails_on_missing_machine(self):
        machines = rows()
        machines[cutover.APPS[1]].pop()

        def fake_fly(*args):
            return json.dumps(machines[args[3]])

        with patch.object(cutover, "fly", side_effect=fake_fly):
            with self.assertRaisesRegex(RuntimeError, "empty Machine inventory"):
                cutover.inventory()

    def test_identity_hash_ignores_state_but_rejects_image_change(self):
        machines = rows()

        def fake_fly(*args):
            return json.dumps(machines[args[3]])

        with patch.object(cutover, "fly", side_effect=fake_fly):
            original = cutover.inventory()["identity_sha256"]
            machines[cutover.APPS[0]][0]["state"] = "stopped"
            self.assertEqual(original, cutover.inventory()["identity_sha256"])
            machines[cutover.APPS[0]][0]["image_ref"] = "registry.fly.io/changed"
            self.assertNotEqual(original, cutover.inventory()["identity_sha256"])

    def test_quiesce_disables_autostart_and_preserves_before_receipt(self):
        machines = rows()
        calls = []

        def fake_fly(*args):
            if args[:2] == ("machine", "list"):
                return json.dumps(machines[args[3]])
            calls.append(args)
            machine_id = args[2]
            app = args[4]
            machine = next(row for row in machines[app] if row["id"] == machine_id)
            if args[1] == "update":
                machine["config"]["services"][0]["autostart"] = False
                machine["state"] = "created"
            elif args[1] == "stop":
                machine["state"] = "stopped"
            return ""

        with patch.object(cutover, "fly", side_effect=fake_fly):
            before = cutover.inventory()
            cutover.quiesce(before)
            after = cutover.inventory()
        self.assertEqual(before["identity_sha256"], after["identity_sha256"])
        self.assertEqual(len(calls), 4)
        self.assertTrue(all("--autostart=false" in call for call in calls))
        self.assertTrue(
            all(
                machine["state"] == "created"
                and machine["service_autostart"] == [False]
                for app in after["apps"].values()
                for machine in app
            )
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            cutover.write_receipt(path, before)
            self.assertEqual(
                json.loads(path.read_text())["identity_sha256"],
                before["identity_sha256"],
            )
            with self.assertRaisesRegex(RuntimeError, "already exists"):
                cutover.write_receipt(path, before)


if __name__ == "__main__":
    unittest.main()
