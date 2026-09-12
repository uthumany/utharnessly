"""Enumerate and exercise every advertised CLI help path without mutations."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
binary = str(root / 'target/release/utharness')
paths = []
with tempfile.TemporaryDirectory(prefix='utharness-cli-audit-') as workspace:
    environment = dict(os.environ, UTHARNESS_HOME=workspace, UTHARNESS_PROVIDER='offline', NO_COLOR='1')
    def visit(arguments):
        result = subprocess.run([binary, *arguments, '--help'], cwd=workspace, env=environment,
                                capture_output=True, text=True, timeout=30)
        assert result.returncode == 0, (arguments, result.stderr)
        assert 'Usage:' in result.stdout, arguments
        paths.append('utharness ' + ' '.join(arguments))
        section = re.search(r'Commands:\n(.*?)(?:\n\S|\Z)', result.stdout, re.S)
        if section:
            for command in re.findall(r'^  ([a-z][a-z0-9-]*)\s', section.group(1), re.M):
                if command != 'help':
                    visit([*arguments, command])
    visit([])
print(json.dumps({'help_paths_passed': len(paths), 'paths': paths}, indent=2))
