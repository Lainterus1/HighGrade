def normalize_items(items):
    normalized = (item.strip() for item in items)
    return [item for item in normalized if item]
