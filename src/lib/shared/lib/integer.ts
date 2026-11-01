export type IntegerValidation =
  | { status: "valid"; value: number }
  | { status: "invalid"; reason: "required" | "integer" | "range" };

export function validateInteger(text: string, min: number, max: number): IntegerValidation {
  if (text.trim() === "") return { status: "invalid", reason: "required" };
  const value = Number(text);
  if (!Number.isSafeInteger(value)) return { status: "invalid", reason: "integer" };
  if (value < min || value > max) return { status: "invalid", reason: "range" };
  return { status: "valid", value };
}
