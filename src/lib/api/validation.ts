export interface ValueLocation {
  path: string;
}

export interface ValidationIssue extends ValueLocation {
  expected: string;
}

/** Reports the field, never the received value (which can contain secrets). */
export class ResponseValidationError extends Error {
  constructor(readonly issue: ValidationIssue, readonly command?: string) {
    super("Некорректный ответ" + (command ? " " + command : "") + ": " + issue.path + "; ожидалось " + issue.expected);
    this.name = "ResponseValidationError";
  }
}

function invalid(path: string, expected: string): never {
  throw new ResponseValidationError({ path, expected });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function readObject(value: unknown, path: string): Record<string, unknown> {
  return isRecord(value) ? value : invalid(path, "объект");
}

export function readString(value: unknown, path: string): string {
  return typeof value === "string" ? value : invalid(path, "строка");
}

export function readBoolean(value: unknown, path: string): boolean {
  return typeof value === "boolean" ? value : invalid(path, "boolean");
}

export function readNonnegativeNumber(value: unknown, path: string): number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0
    ? value : invalid(path, "конечное неотрицательное число");
}

export function readInteger(value: unknown, path: string): number {
  const number = readNonnegativeNumber(value, path);
  return Number.isSafeInteger(number) ? number : invalid(path, "неотрицательное целое число");
}

export function readArray(value: unknown, path: string): unknown[] {
  return Array.isArray(value) ? value : invalid(path, "массив");
}

export function readOptional<T>(
  value: unknown,
  path: string,
  parse: (value: unknown, path: string) => T
): T | null | undefined {
  return value === null || value === undefined ? value : parse(value, path);
}
