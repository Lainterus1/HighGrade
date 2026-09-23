"""Validation for our bundled schema subset (not a general JSON Schema engine)."""
from __future__ import annotations
import json
import math
import re
from pathlib import Path

DATA = Path(__file__).resolve().parents[1] / 'data'


def validate(value, schema, document=None, location='$'):
    document = document or schema
    supported = {'$schema', '$id', '$defs', 'title', 'description', '$ref', 'type',
                 'const', 'enum', 'required', 'properties', 'additionalProperties',
                 'items', 'minItems', 'minLength', 'pattern', 'minimum', 'maximum', 'allOf'}
    if set(schema) - supported:
        raise ValueError(f'{location}: unsupported bundled schema keywords {set(schema) - supported}')
    if '$ref' in schema:
        target = document
        if not schema['$ref'].startswith('#/'):
            raise ValueError('only local schema references are supported')
        for part in schema['$ref'][2:].split('/'):
            target = target[part]
        validate(value, target, document, location)
    for part in schema.get('allOf', []):
        validate(value, part, document, location)
    types = schema.get('type', [])
    types = [types] if isinstance(types, str) else types
    actual = ('null' if value is None else 'boolean' if isinstance(value, bool) else
              'integer' if isinstance(value, int) else 'number' if isinstance(value, float) else
              'string' if isinstance(value, str) else 'array' if isinstance(value, list) else
              'object' if isinstance(value, dict) else 'unknown')
    if types and actual not in types and not (actual == 'integer' and 'number' in types):
        raise ValueError(f'{location}: expected {types}, got {actual}')
    if 'const' in schema and value != schema['const']:
        raise ValueError(f'{location}: invalid constant')
    if 'enum' in schema and value not in schema['enum']:
        raise ValueError(f'{location}: unsupported value {value!r}')
    if actual in {'integer', 'number'}:
        if not math.isfinite(value) or value < schema.get('minimum', -math.inf) or value > schema.get('maximum', math.inf):
            raise ValueError(f'{location}: number is non-finite or out of bounds')
    if actual == 'string':
        if len(value) < schema.get('minLength', 0) or ('pattern' in schema and not re.search(schema['pattern'], value)):
            raise ValueError(f'{location}: invalid string')
    if actual == 'object':
        if set(schema.get('required', [])) - set(value):
            raise ValueError(f'{location}: missing {sorted(set(schema["required"]) - set(value))}')
        properties = schema.get('properties', {})
        for key, item in value.items():
            rule = properties.get(key, schema.get('additionalProperties', True))
            if rule is False:
                raise ValueError(f'{location}.{key}: unsupported field')
            if isinstance(rule, dict):
                validate(item, rule, document, f'{location}.{key}')
    if actual == 'array':
        if len(value) < schema.get('minItems', 0):
            raise ValueError(f'{location}: array is too short')
        for index, item in enumerate(value):
            validate(item, schema.get('items', {}), document, f'{location}[{index}]')


def schema_for(name):
    return json.loads((DATA / f'{name}.schema.json').read_text(encoding='utf-8'))


def safe_relative(value):
    if not isinstance(value, str) or not value or '\\' in value or ':' in value or value.startswith('/') or '..' in value.split('/'):
        raise ValueError(f'unsafe repository-relative path: {value!r}')
    return value


def load_policy(override=None):
    from .core import deep_merge
    policy = json.loads((DATA / 'default-policy.json').read_text(encoding='utf-8'))
    if override:
        raw = json.loads(Path(override).read_text(encoding='utf-8'))
        if not isinstance(raw, dict):
            raise ValueError('policy override must be an object')
        policy = deep_merge(policy, raw)
    validate(policy, schema_for('policy'))
    for key in ('include_paths', 'excluded_paths', 'source_roots', 'test_paths', 'excluded_directories'):
        for path in policy[key]:
            safe_relative(path)
    for metric, rule in policy['metrics'].items():
        levels = [rule[name] for name in ('watch', 'refactor', 'critical')]
        if levels != sorted(levels, reverse=rule['direction'] == 'low'):
            raise ValueError(f'{metric}: thresholds must follow severity order')
        if rule['unit'] == 'percent' and any(value > 100 for value in levels):
            raise ValueError(f'{metric}: percentage threshold exceeds 100')
    return policy


def validate_report(report):
    validate(report, schema_for('report'))
    seen = set()
    for item in report['measurements']:
        if item['measurement_id'] in seen:
            raise ValueError(f'duplicate measurement ID: {item["measurement_id"]}')
        seen.add(item['measurement_id'])
        safe_relative(item['path'])
        if item['unit'] == 'percent' and not 0 <= item['value'] <= 100:
            raise ValueError('percentage measurement out of bounds')
        if item.get('start_line') and item.get('end_line') is not None and item['end_line'] < item['start_line']:
            raise ValueError('invalid measurement line range')
