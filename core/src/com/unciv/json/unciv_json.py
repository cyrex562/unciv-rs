import json
from typing import Type, TypeVar, Any
from datetime import timedelta
from pathlib import Path
import os

T = TypeVar('T')

class UncivJson:
    """JSON serialization utilities with custom serializers."""
    
    def __init__(self):
        self._encoders = {}
        self._decoders = {}
        self._ignore_deprecated = True
        self._ignore_unknown_fields = True
        
        # Register default serializers
        self.register_serializer(timedelta, self._duration_encoder, self._duration_decoder)
    
    def register_serializer(self, cls: Type, encoder: callable, decoder: callable) -> None:
        """Register custom serializer/deserializer for a type."""
        self._encoders[cls] = encoder
        self._decoders[cls] = decoder
    
    def _duration_encoder(self, obj: timedelta) -> str:
        """Encode timedelta to ISO 8601 duration string."""
        return str(obj)
    
    def _duration_decoder(self, obj: str) -> timedelta:
        """Decode ISO 8601 duration string to timedelta."""
        return timedelta.fromisoformat(obj)
    
    def _custom_encoder(self, obj: Any) -> Any:
        """Custom JSON encoder that handles registered types."""
        if type(obj) in self._encoders:
            return self._encoders[type(obj)](obj)
        raise TypeError(f"Object of type {type(obj)} is not JSON serializable")
    
    def _custom_decoder(self, obj: Any) -> Any:
        """Custom JSON decoder that handles registered types."""
        if isinstance(obj, str):
            # Try to decode as registered type
            for decoder in self._decoders.values():
                try:
                    return decoder(obj)
                except (ValueError, TypeError):
                    continue
        return obj
    
    def dumps(self, obj: Any) -> str:
        """Serialize object to JSON string."""
        return json.dumps(obj, default=self._custom_encoder)
    
    def loads(self, json_str: str) -> Any:
        """Deserialize JSON string to object."""
        return json.loads(json_str, object_hook=self._custom_decoder)
    
    def from_json_file(self, cls: Type[T], file_path: str) -> T:
        """Load JSON from file and deserialize to specified type.
        
        Args:
            cls: The type to deserialize to
            file_path: Path to the JSON file
            
        Returns:
            Deserialized object of type cls
            
        Raises:
            Exception: If file cannot be read or JSON cannot be parsed
        """
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                json_text = f.read()
            return self.loads(json_text)
        except Exception as e:
            raise Exception(f"Could not parse json of file {os.path.basename(file_path)}") from e

# Create a singleton instance with default configuration
json_utils = UncivJson() 