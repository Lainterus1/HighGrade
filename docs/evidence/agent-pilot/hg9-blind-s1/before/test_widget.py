import unittest

from widget import render


class WidgetTests(unittest.TestCase):
    def test_render(self):
        self.assertEqual(render("demo"), "<demo>")
