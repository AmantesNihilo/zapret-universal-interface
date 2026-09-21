/** Pause visual work in background windows without stopping service events. */
export function bindUiActivity() {
  const root = document.documentElement;
  const update = () => {
    root.toggleAttribute("data-ui-inactive", document.hidden || !document.hasFocus());
  };

  document.addEventListener("visibilitychange", update);
  window.addEventListener("focus", update);
  window.addEventListener("blur", update);
  update();

  return () => {
    document.removeEventListener("visibilitychange", update);
    window.removeEventListener("focus", update);
    window.removeEventListener("blur", update);
    root.removeAttribute("data-ui-inactive");
  };
}
