import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushSync, mount, unmount } from "svelte";
import { validateInteger } from "$lib/shared/lib/integer";
import IntegerField from "$lib/shared/ui/IntegerField.svelte";
import { settings } from "$lib/stores/settings";

let target: HTMLDivElement;
let component: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  settings.update(value => ({ ...value, language: "ru" }));
  target = document.createElement("div");
  document.body.append(target);
});
afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  target.remove();
});

function render(onCommit = vi.fn<(value: number) => void | Promise<void>>()) {
  component = mount(IntegerField, {
    target, props: { value: 1443, min: 1, max: 65535, resetKey: "profile-a", label: "Порт", onCommit }
  });
  flushSync();
  const input = target.querySelector("input");
  if (!input) throw new Error("Input missing");
  const type = (text: string) => {
    input.value = text;
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
  };
  const blur = () => { input.dispatchEvent(new FocusEvent("blur")); flushSync(); };
  return { input, type, blur, onCommit };
}

describe("integer draft and validation", () => {
  it.each(["", " ", "NaN", "Infinity", "1.5", "-1", "0", "65536"])("rejects invalid port %j", text => {
    expect(validateInteger(text, 1, 65535).status).toBe("invalid");
  });
  it.each([0, 64])("allows the pool boundary %i", value => {
    expect(validateInteger(String(value), 0, 64)).toEqual({ status: "valid", value });
  });
  it("keeps intermediate text local and commits only on blur", async () => {
    const h = render();
    h.type("8"); h.type("80"); h.type("8080");
    expect(h.onCommit).not.toHaveBeenCalled();
    h.blur();
    await vi.waitFor(() => expect(h.onCommit).toHaveBeenCalledExactlyOnceWith(8080));
  });
  it.each(["", "1.5", "0", "65536"])("shows a field error and never saves %j", text => {
    const h = render();
    h.type(text); h.blur();
    expect(target.querySelector('[role="alert"]')).not.toBeNull();
    expect(h.input.getAttribute("aria-invalid")).toBe("true");
    expect(h.onCommit).not.toHaveBeenCalled();
  });
  it("does not save an unchanged value", () => {
    const h = render();
    h.blur();
    expect(h.onCommit).not.toHaveBeenCalled();
  });
  it("joins Enter and blur while save is pending", async () => {
    let finish: (() => void) | undefined;
    const pending = new Promise<void>(resolve => { finish = resolve; });
    const h = render(vi.fn().mockReturnValue(pending));
    h.type("8080");
    h.input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    h.blur();
    expect(h.onCommit).toHaveBeenCalledTimes(1);
    expect(h.input.disabled).toBe(true);
    finish?.();
    await vi.waitFor(() => expect(h.input.disabled).toBe(false));
  });
  it("shows save failure and allows correction and retry", async () => {
    const onCommit = vi.fn<(value: number) => Promise<void>>()
      .mockRejectedValueOnce(new Error("disk unavailable")).mockResolvedValueOnce();
    const h = render(onCommit);
    h.type("8080"); h.blur();
    await vi.waitFor(() => expect(target.textContent).toContain("disk unavailable"));
    h.type("9090"); h.blur();
    await vi.waitFor(() => expect(onCommit).toHaveBeenCalledTimes(2));
    await vi.waitFor(() => expect(target.querySelector('[role="alert"]')).toBeNull());
  });
});
