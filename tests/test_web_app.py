"""Offline tests for the Uclaw web UI."""
import os
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "web"))

from fastapi.testclient import TestClient

import app as web_app


class TestHomeRoute(unittest.TestCase):
    def test_home_serves_from_any_working_directory(self):
        # Regression: the page used to be opened as "static/index.html", which
        # only resolves when the process is launched from the web/ directory.
        original_cwd = os.getcwd()
        os.chdir("/")
        try:
            client = TestClient(web_app.app)
            resp = client.get("/")
        finally:
            os.chdir(original_cwd)
        self.assertEqual(resp.status_code, 200)
        self.assertIn("<", resp.text)


if __name__ == "__main__":
    unittest.main()
