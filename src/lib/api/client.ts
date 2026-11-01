import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { ResponseValidationError } from "./validation";

/** A type argument is not validation: a decoder must check the unknown response. */
export async function invokeValidated<T>(
  command: string,
  parse: (value: unknown) => T,
  args?: InvokeArgs
): Promise<T> {
  const raw = await invoke<unknown>(command, args);
  try {
    return parse(raw);
  } catch (caught: unknown) {
    if (caught instanceof ResponseValidationError) {
      throw new ResponseValidationError(caught.issue, command);
    }
    throw caught;
  }
}
