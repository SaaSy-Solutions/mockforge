"""Behavioral guards for the real bundled-demo HTTP verifier."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("demo", ROOT / "scripts/verify-bundled-demo.py")
demo = importlib.util.module_from_spec(spec)
spec.loader.exec_module(demo)


class DemoResponses(unittest.TestCase):
    def test_unexpanded_marker_rejected_inside_nested_response(self):
        for value in ("{{faker.commerce.price}}", "{{rand.int 0 100}}", "unfinished {{thing", "thing}}"):
            with self.subTest(value=value), self.assertRaisesRegex(AssertionError, "Unexpanded template"):
                demo.no_markers({"products": [{"price": value}]})
        demo.no_markers({"products": [{"price": "23.99"}]})
        with self.assertRaisesRegex(AssertionError, "Unexpanded template"):
            demo.no_markers({"{{unexpected.key}}": "value"})

    def test_numeric_strings_keep_the_bundled_json_contract(self):
        demo.same_shape({"stock": "{{rand.int 0 100}}"}, {"stock": "42"})
        with self.assertRaisesRegex(AssertionError, "JSON type changed"):
            demo.same_shape({"stock": "{{rand.int 0 100}}"}, {"stock": 42})
        with self.assertRaisesRegex(AssertionError, "JSON fields changed"):
            demo.same_shape({"stock": "42"}, {"other": "42"})

    def test_weather_requires_usable_values_beyond_no_markers(self):
        weather = {"temperature": "12", "humidity": "40", "windSpeed": "3", "condition": "sunny",
                   "location": {"city": "London", "country": "United Kingdom",
                                "coordinates": {"lat": "51.5074", "lon": "-0.1278"}}}
        demo.semantic_values("/api/weather/current", weather)
        weather["temperature"] = "50"
        with self.assertRaisesRegex(AssertionError, "outside"):
            demo.semantic_values("/api/weather/current", weather)

    def test_covers_all_six_bundled_json_response_examples(self):
        cases, files = demo.examples()
        self.assertEqual(set(files), {"ecommerce-store", "weather-geo", "chat-api"})
        self.assertEqual({(method, path, status) for method, path, _, status, _ in cases}, {
            ("GET", "/api/products", 200), ("GET", "/api/cart", 200), ("POST", "/api/orders", 201),
            ("GET", "/api/weather/current", 200), ("GET", "/api/weather/forecast", 200),
            ("GET", "/api/messages", 200)})


if __name__ == "__main__":
    unittest.main()
