"""Pass explicit file targets to Pylint without OS command-line length limits."""
import json
from pathlib import Path
import sys
from pylint.lint import Run

request = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
sys.path[:0] = request['source_roots']
result = Run(request['args'], exit=False)
raise SystemExit(result.linter.msg_status)
