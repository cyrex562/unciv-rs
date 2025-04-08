from typing import Dict, Any, Optional
import json
from dataclasses import dataclass
from math import Vector2  # Assuming you have a Vector2 implementation

@dataclass
class LastSeenImprovement:
    _map: Dict[Vector2, str] = None

    def __init__(self):
        self._map = {}

    def __getitem__(self, key: Vector2) -> str:
        return self._map[key]

    def __setitem__(self, key: Vector2, value: str) -> None:
        self._map[key] = value

    def __delitem__(self, key: Vector2) -> None:
        del self._map[key]

    def __iter__(self):
        return iter(self._map)

    def __len__(self) -> int:
        return len(self._map)

    def to_json(self) -> str:
        """Serialize the map to JSON string."""
        result = {}
        for key, value in self._map.items():
            # Convert Vector2 to string representation
            key_str = f"({key.x},{key.y})"
            result[key_str] = value
        return json.dumps(result)

    @classmethod
    def from_json(cls, json_str: str) -> 'LastSeenImprovement':
        """Deserialize from JSON string."""
        instance = cls()
        data = json.loads(json_str)

        # Handle old format if present
        if isinstance(data, dict) and "class" in data and data["class"] == "com.unciv.json.HashMapVector2":
            return cls._read_old_format(data)

        # Handle current format
        for key_str, value in data.items():
            try:
                # Parse Vector2 from string
                key = cls._parse_vector2(key_str)
                instance[key] = value
            except ValueError:
                continue
        return instance

    @staticmethod
    def _parse_vector2(s: str) -> Vector2:
        """Parse a string in format '(x,y)' into a Vector2."""
        try:
            # Remove parentheses and split by comma
            x, y = s.strip('()').split(',')
            return Vector2(float(x), float(y))
        except (ValueError, AttributeError):
            raise ValueError(f"Invalid Vector2 string format: {s}")

    @staticmethod
    def _read_old_format(data: Dict[str, Any]) -> 'LastSeenImprovement':
        """Handle backward compatibility with old format."""
        instance = LastSeenImprovement()
        for entry in data.get("entries", []):
            if len(entry) >= 2:
                key = Vector2(entry[0][0], entry[0][1])  # Assuming old format stores Vector2 as [x, y]
                value = entry[1]
                instance[key] = value
        return instance

    def __eq__(self, other: Any) -> bool:
        if isinstance(other, LastSeenImprovement):
            return self._map == other._map
        if isinstance(other, dict):
            return self._map == other
        return False

    def __hash__(self) -> int:
        return hash(tuple(sorted(self._map.items())))