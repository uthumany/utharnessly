import platform
import sys
import unittest
from unittest.mock import patch

from utharnessly import __version__
from utharnessly.cli import VERSION, platform_asset


class LauncherTests(unittest.TestCase):
    def test_version_is_consistent(self):
        self.assertEqual(VERSION, __version__)

    @patch.object(platform, "machine", return_value="x86_64")
    @patch.object(sys, "platform", "linux")
    def test_linux_x64_asset(self, _machine):
        self.assertEqual(platform_asset(), ("utharnessly-linux-x64.tar.gz", "tar.gz"))

    @patch.object(platform, "machine", return_value="AMD64")
    @patch.object(sys, "platform", "win32")
    def test_windows_x64_asset(self, _machine):
        self.assertEqual(platform_asset(), ("utharnessly-windows-x64.zip", "zip"))

    @patch.object(platform, "machine", return_value="aarch64")
    @patch.object(sys, "platform", "linux")
    def test_unsupported_architecture_is_explicit(self, _machine):
        with self.assertRaisesRegex(RuntimeError, "No published utharnessly binary"):
            platform_asset()


if __name__ == "__main__":
    unittest.main()
