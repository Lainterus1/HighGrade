import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from pathlib import Path
from app import normalize_items
s=Path("README.md").read_text()
assert '[" A ", "", " B "] -> ["A", "B"]' in s
assert normalize_items([" A ", "", " B "]) == ["A", "B"]
print("README example agrees with the implementation")
