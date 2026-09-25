function evictOldestOverCap(map, cap) {
  while (map.size > cap) map.delete(map.keys().next().value);
}

export { evictOldestOverCap };