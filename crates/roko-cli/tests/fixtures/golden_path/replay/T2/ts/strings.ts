/** `text` with its characters in reverse order. */
export function reverse(text: string): string {
  return [...text].reverse().join("");
}

/** `text` in camel case: words split on spaces, hyphens and underscores. */
export function camelCase(text: string): string {
  return text
    .split(/[\s_-]+/)
    .filter((word) => word.length > 0)
    .map((word, index) => {
      const lower = word.toLowerCase();
      return index === 0 ? lower : lower.charAt(0).toUpperCase() + lower.slice(1);
    })
    .join("");
}
