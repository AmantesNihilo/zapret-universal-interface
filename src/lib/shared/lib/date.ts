export function formatUnixTime(value: string, language: string) {
  const timestamp = Number(value);
  if (!Number.isFinite(timestamp) || timestamp <= 0) return value;
  return new Date(timestamp * 1000).toLocaleString(language === "ru" ? "ru-RU" : "en-US");
}
