"""Local-engine admission must precede any Qdrant fixture mutation."""
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'ci'))
from qdrant_fixture import QdrantFixture


class LocalEngineTests(unittest.TestCase):
    def fixture(self, context, info=None):
        fixture = QdrantFixture('qdrant/qdrant:v1.19.2', {})
        def command(*args):
            if args == ('context', 'inspect'):
                return json.dumps(context)
            if args[0] == 'info':
                return json.dumps(info)
            return ''
        fixture.docker = Mock(side_effect=command)
        fixture.refresh_address = Mock()
        return fixture

    @patch('qdrant_fixture.subprocess.run')
    def test_local_podman_without_docker_context_is_accepted(self, run):
        fixture = self.fixture([], {'host': {'serviceIsRemote': False}})
        fixture.start()
        self.assertTrue(fixture.local_context)
        run.assert_called_once()
        self.assertIn('127.0.0.1::6333', run.call_args.args[0])

    @patch('qdrant_fixture.subprocess.run')
    def test_unix_docker_context_is_accepted(self, run):
        fixture = self.fixture([{'Endpoints': {'docker': {'Host': 'unix:///run/docker.sock'}}}])
        fixture.start()
        self.assertTrue(fixture.local_context)
        run.assert_called_once()

    def test_remote_or_unknown_engines_never_mutate_resources(self):
        cases = [
            ([{'Endpoints': {'docker': {'Host': 'ssh://remote'}}}], None),
            ([], {'host': {'serviceIsRemote': True}}),
            ([], {'host': {}}),
            ([], {}),
        ]
        for context, info in cases:
            with self.subTest(context=context, info=info):
                fixture = self.fixture(context, info)
                with self.assertRaises(RuntimeError):
                    fixture.start()
                self.assertFalse(fixture.local_context)
                self.assertTrue(fixture.close())
                self.assertFalse(any(call.args[0] in ('pull', 'volume', 'run') for call in fixture.docker.call_args_list))


if __name__ == '__main__':
    unittest.main()
