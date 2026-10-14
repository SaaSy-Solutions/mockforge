"""Exercise release ordering against the actual workspace dependency graph."""
import importlib.util
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("publish_order", ROOT / "scripts/check_publish_order.py")
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


class PublishOrderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=ROOT,
        ))
        cls.names = validator.read_publish_order((ROOT / "scripts/publish-crates.sh").read_text())

    def test_current_list_covers_publishable_workspace_and_orders_dependencies(self):
        publishable = {p["name"] for p in self.metadata["packages"]
                       if p["name"].startswith("mockforge-") and p.get("publish") != []}
        # The publisher also explicitly ships required internal publish=false crates.
        self.assertLessEqual(publishable, set(self.names))
        self.assertEqual(validator.ordering_errors(self.metadata, self.names), [])

    def test_actual_0233_failure_order_is_rejected_for_all_three_brokers(self):
        old_order = [name for name in self.names if name != "mockforge-recorder"]
        old_order.insert(old_order.index("mockforge-pipelines") + 1, "mockforge-recorder")
        self.assertEqual(validator.ordering_errors(self.metadata, old_order), [
            "mockforge-recorder must precede mockforge-amqp",
            "mockforge-recorder must precede mockforge-kafka",
            "mockforge-recorder must precede mockforge-mqtt",
        ])


if __name__ == "__main__":
    unittest.main()
