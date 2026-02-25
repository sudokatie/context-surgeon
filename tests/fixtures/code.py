#!/usr/bin/env python3
"""
Example Python module demonstrating various language features.

This module contains classes and functions for data processing,
illustrating common patterns in Python development.
"""

# MIT License
# Copyright (c) 2024 Example Corp
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files.

import os
import sys
import json
import logging
from typing import List, Dict, Optional, Any, TypeVar, Generic
from dataclasses import dataclass, field
from abc import ABC, abstractmethod
from collections import defaultdict
from functools import lru_cache
from pathlib import Path

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

T = TypeVar('T')


@dataclass
class Config:
    """Application configuration settings."""
    
    debug: bool = False
    max_retries: int = 3
    timeout: float = 30.0
    base_url: str = "https://api.example.com"
    cache_dir: Path = field(default_factory=lambda: Path.home() / ".cache" / "myapp")
    
    def __post_init__(self):
        """Ensure cache directory exists."""
        self.cache_dir.mkdir(parents=True, exist_ok=True)


class DataProcessor(ABC):
    """Abstract base class for data processors."""
    
    @abstractmethod
    def process(self, data: Any) -> Any:
        """Process input data and return result."""
        pass
    
    @abstractmethod
    def validate(self, data: Any) -> bool:
        """Validate input data before processing."""
        pass


class JsonProcessor(DataProcessor):
    """Process JSON data with validation and transformation."""
    
    def __init__(self, schema: Optional[Dict] = None):
        self.schema = schema or {}
        self._cache: Dict[str, Any] = {}
    
    def process(self, data: Any) -> Dict[str, Any]:
        """
        Process JSON data.
        
        Args:
            data: Raw input data (string or dict)
            
        Returns:
            Processed dictionary
            
        Raises:
            ValueError: If data is invalid
        """
        if isinstance(data, str):
            try:
                data = json.loads(data)
            except json.JSONDecodeError as e:
                logger.error(f"Failed to parse JSON: {e}")
                raise ValueError(f"Invalid JSON: {e}")
        
        if not self.validate(data):
            raise ValueError("Data validation failed")
        
        return self._transform(data)
    
    def validate(self, data: Any) -> bool:
        """Validate data against schema."""
        if not isinstance(data, dict):
            return False
        
        for key, expected_type in self.schema.items():
            if key in data and not isinstance(data[key], expected_type):
                logger.warning(f"Type mismatch for {key}")
                return False
        
        return True
    
    def _transform(self, data: Dict) -> Dict[str, Any]:
        """Apply transformations to data."""
        result = dict(data)
        result['_processed'] = True
        result['_timestamp'] = self._get_timestamp()
        return result
    
    @staticmethod
    def _get_timestamp() -> str:
        """Get current timestamp."""
        from datetime import datetime
        return datetime.utcnow().isoformat()


class Cache(Generic[T]):
    """Generic caching mechanism with LRU eviction."""
    
    def __init__(self, max_size: int = 100):
        self.max_size = max_size
        self._data: Dict[str, T] = {}
        self._access_order: List[str] = []
    
    def get(self, key: str) -> Optional[T]:
        """Retrieve value from cache."""
        if key in self._data:
            self._access_order.remove(key)
            self._access_order.append(key)
            return self._data[key]
        return None
    
    def set(self, key: str, value: T) -> None:
        """Store value in cache with LRU eviction."""
        if key in self._data:
            self._access_order.remove(key)
        elif len(self._data) >= self.max_size:
            oldest = self._access_order.pop(0)
            del self._data[oldest]
        
        self._data[key] = value
        self._access_order.append(key)
    
    def clear(self) -> None:
        """Clear all cached data."""
        self._data.clear()
        self._access_order.clear()


@lru_cache(maxsize=128)
def fibonacci(n: int) -> int:
    """
    Calculate the nth Fibonacci number.
    
    Uses memoization for efficiency.
    
    Args:
        n: Position in Fibonacci sequence
        
    Returns:
        The nth Fibonacci number
    """
    if n < 2:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)


def process_file(path: Path, processor: DataProcessor) -> List[Dict]:
    """
    Process a file using the given processor.
    
    Args:
        path: Path to input file
        processor: DataProcessor instance
        
    Returns:
        List of processed records
    """
    results = []
    
    with open(path, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            
            try:
                result = processor.process(line)
                results.append(result)
            except ValueError as e:
                logger.warning(f"Line {line_num}: {e}")
    
    return results


def batch_process(
    items: List[Any],
    batch_size: int = 10,
    processor: Optional[DataProcessor] = None
) -> List[Any]:
    """
    Process items in batches.
    
    Args:
        items: List of items to process
        batch_size: Number of items per batch
        processor: Optional processor to apply
        
    Returns:
        List of processed items
    """
    results = []
    
    for i in range(0, len(items), batch_size):
        batch = items[i:i + batch_size]
        logger.info(f"Processing batch {i // batch_size + 1}")
        
        for item in batch:
            if processor:
                item = processor.process(item)
            results.append(item)
    
    return results


class Pipeline:
    """Data processing pipeline with multiple stages."""
    
    def __init__(self):
        self.stages: List[DataProcessor] = []
        self.config = Config()
    
    def add_stage(self, processor: DataProcessor) -> 'Pipeline':
        """Add a processing stage to the pipeline."""
        self.stages.append(processor)
        return self
    
    def run(self, data: Any) -> Any:
        """Run data through all pipeline stages."""
        result = data
        
        for i, stage in enumerate(self.stages):
            logger.info(f"Running stage {i + 1}/{len(self.stages)}")
            result = stage.process(result)
        
        return result


def main():
    """Main entry point."""
    config = Config(debug=True)
    logger.info(f"Starting with config: {config}")
    
    # Create processor
    schema = {'name': str, 'value': (int, float)}
    processor = JsonProcessor(schema)
    
    # Process sample data
    sample = '{"name": "test", "value": 42}'
    result = processor.process(sample)
    
    print(f"Result: {json.dumps(result, indent=2)}")
    
    # Test cache
    cache: Cache[str] = Cache(max_size=10)
    cache.set("key1", "value1")
    print(f"Cached: {cache.get('key1')}")
    
    # Test fibonacci
    for n in range(10):
        print(f"fib({n}) = {fibonacci(n)}")


if __name__ == "__main__":
    main()
