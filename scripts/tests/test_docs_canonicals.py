import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from inject_docs_canonicals import inject


class CanonicalLinks(unittest.TestCase):
    def test_repeated_injection_is_identical(self):
        page = '<html><head>\n    <title>Docs</title>\n</head><body></body></html>'
        first = inject(page, 'https://docs.example/guide/')
        self.assertEqual(inject(first, 'https://docs.example/guide/'), first)

    def test_replaces_existing_url_and_escapes_attribute(self):
        first = inject('<head></head>', 'https://old.example/')
        changed = inject(first, 'https://new.example/?a=1&b=2')
        self.assertNotIn('old.example', changed)
        self.assertEqual(changed.count('rel="canonical"'), 1)
        self.assertIn('a=1&amp;b=2', changed)

    def test_rejects_non_html(self):
        with self.assertRaises(ValueError):
            inject('missing head', 'https://docs.example/')


if __name__ == '__main__':
    unittest.main()
