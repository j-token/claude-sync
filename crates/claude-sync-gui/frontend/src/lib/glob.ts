function globToRegExp(pattern: string): RegExp {
  const escaped = pattern
    .split("*")
    .map((part) => part.replace(/[.+?^${}()|[\]\\/]/g, "\\$&"))
    .join(".*");
  return new RegExp(`^${escaped}$`);
}

export function matchesGlob(id: string, pattern: string): boolean {
  return globToRegExp(pattern.trim()).test(id);
}

export function matchesTemplate(id: string, include: string[], exclude: string[]): boolean {
  const isIncluded = include.some((pattern) => matchesGlob(id, pattern));
  const isExcluded = exclude.some((pattern) => matchesGlob(id, pattern));
  return isIncluded && !isExcluded;
}

export function filterByTemplate(ids: string[], include: string[], exclude: string[]): string[] {
  return ids.filter((id) => matchesTemplate(id, include, exclude));
}
