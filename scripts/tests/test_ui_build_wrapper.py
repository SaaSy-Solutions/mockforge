#!/usr/bin/env python3
"""Exercise the actual UI build wrapper with an owned package-manager fixture."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class UiBuildWrapperTests(unittest.TestCase):
    def run_wrapper(self, exit_code):
        with tempfile.TemporaryDirectory(prefix="ui build wrapper ") as temporary:
            directory = Path(temporary)
            project = directory / "package"
            (project / "ui").mkdir(parents=True)
            script = project / "build_ui.sh"
            script.write_bytes((ROOT / "crates/mockforge-ui/build_ui.sh").read_bytes())
            fake_bin = directory / "bin"
            fake_bin.mkdir()
            fake = fake_bin / "corepack"
            fake.write_text("#!/usr/bin/env python3\nimport json,os,sys\nfrom pathlib import Path\nPath(os.environ['UI_WRAPPER_RECORD']).write_text(json.dumps({'cwd':os.getcwd(),'arguments':sys.argv[1:]}))\nsys.exit(int(os.environ['UI_WRAPPER_EXIT']))\n")
            fake.chmod(0o755)
            record = directory / "record.json"
            elsewhere = directory / "elsewhere"
            elsewhere.mkdir()
            env = {"PATH":str(fake_bin)+os.pathsep+os.environ["PATH"],
                "UI_WRAPPER_RECORD":str(record),"UI_WRAPPER_EXIT":str(exit_code)}
            result = subprocess.run(["bash",str(script)],cwd=elsewhere,env=env,
                text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=5)
            self.assertTrue(record.exists(), "package manager was not invoked")
            invocation = json.loads(record.read_text())
            self.assertEqual(invocation["arguments"],["pnpm","run","build"])
            self.assertEqual(invocation["cwd"],str(project/"ui"))
            return result

    def test_failed_frontend_build_propagates_without_success_message(self):
        result = self.run_wrapper(37)
        self.assertEqual(result.returncode,37)
        self.assertNotIn("React UI build completed",result.stdout)

    def test_success_uses_declared_manager_and_script_directory(self):
        result = self.run_wrapper(0)
        self.assertEqual(result.returncode,0)
        self.assertIn("React UI build completed",result.stdout)


if __name__ == "__main__":
    unittest.main()
