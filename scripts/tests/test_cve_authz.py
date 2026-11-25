import datetime
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import cve_lab_authz as authz  # noqa: E402

TARGET = "https://lb.example.test"


class FakeStdin:
    def __init__(self, tty):
        self._tty = tty

    def isatty(self):
        return self._tty


class AuthzTest(unittest.TestCase):
    def check_shape(self, result, granted_by):
        self.assertEqual(
            set(result),
            {"authorized", "target", "granted_by", "operator", "attested_at"},
        )
        self.assertIs(result["authorized"], True)
        self.assertEqual(result["target"], TARGET)
        self.assertEqual(result["granted_by"], granted_by)
        stamp = datetime.datetime.fromisoformat(result["attested_at"])
        self.assertEqual(stamp.utcoffset(), datetime.timedelta(0))

    def test_flag(self):
        with mock.patch.dict("os.environ", {}, clear=True):
            result = authz.require_authorization(TARGET, assume_yes=True, operator="ann")
        self.check_shape(result, "flag")
        self.assertEqual(result["operator"], "ann")

    def test_env(self):
        for value in ("1", "true", "YES"):
            with mock.patch.dict("os.environ", {authz.ENV_VAR: value, "USER": "bob"}, clear=True):
                result = authz.require_authorization(TARGET)
            self.check_shape(result, "env")
            self.assertEqual(result["operator"], "bob")

    def test_interactive(self):
        with mock.patch.dict("os.environ", {}, clear=True), mock.patch.object(
            authz.sys, "stdin", FakeStdin(True)
        ), mock.patch("builtins.input", return_value="yes") as prompt:
            result = authz.require_authorization(TARGET)
        self.check_shape(result, "interactive")
        self.assertEqual(result["operator"], "unknown")
        self.assertIn(TARGET, prompt.call_args[0][0])

    def test_refused_without_tty(self):
        with mock.patch.dict("os.environ", {}, clear=True), mock.patch.object(
            authz.sys, "stdin", FakeStdin(False)
        ), mock.patch("sys.stderr"):
            with self.assertRaises(SystemExit) as ctx:
                authz.require_authorization(TARGET)
        self.assertEqual(ctx.exception.code, 2)

    def test_refused_on_wrong_answer_or_env(self):
        env = {authz.ENV_VAR: "0"}
        with mock.patch.dict("os.environ", env, clear=True), mock.patch.object(
            authz.sys, "stdin", FakeStdin(True)
        ), mock.patch("builtins.input", return_value="no"), mock.patch("sys.stderr"):
            with self.assertRaises(SystemExit) as ctx:
                authz.require_authorization(TARGET)
        self.assertEqual(ctx.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
