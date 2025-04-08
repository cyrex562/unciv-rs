import json
from datetime import timedelta
from typing import Any

class DurationSerializer(json.JSONEncoder):
    def default(self, obj: Any) -> Any:
        if isinstance(obj, timedelta):
            return str(obj)
        return super().default(obj)

def duration_decoder(dct: dict) -> Any:
    for key, value in dct.items():
        if isinstance(value, str):
            try:
                # Parse ISO 8601 duration format
                dct[key] = timedelta.fromisoformat(value)
            except ValueError:
                pass
    return dct