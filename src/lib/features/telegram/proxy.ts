export function extractTgProxyLink(message?: string | null) {
  return message?.match(/tg:\/\/proxy\?\S+/)?.[0] ?? null;
}

export function redactProxySecret(value: string) {
  return value.replace(/([?&]secret=)[^&\s]+/gi, "$1<redacted>");
}

export function extractTgProxySecret(link?: string | null) {
  if (!link) return null;
  try {
    return new URL(link).searchParams.get("secret");
  } catch {
    return link.match(/[?&]secret=([^&\s]+)/)?.[1] ?? null;
  }
}

export function previewTgProxySecret(raw: string, generatedLabel: string) {
  const value = raw.trim();
  if (!value) return generatedLabel;
  if (isTelegramLinkSecret(value)) return value;
  if (isHex(value) && (value.length === 32 || value.length === 34)) return `dd${value}`;
  return `dd${plainSecretToHex(value)}`;
}

function isTelegramLinkSecret(value: string) {
  return isHex(value) && ((value.length === 34 && value.startsWith("dd")) || value.startsWith("ee"));
}

function isHex(value: string) {
  return /^[0-9a-fA-F]+$/.test(value);
}

function plainSecretToHex(value: string) {
  const bytes = new Uint8Array(16);
  bytes.set(new TextEncoder().encode(value).slice(0, 16));
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
