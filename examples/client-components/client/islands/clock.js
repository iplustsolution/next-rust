// Custom island module: `hydrate(element, props)` is called by the runtime.
export function hydrate(element, props) {
  const target = element.querySelector(".clock");
  const tick = () => (target.textContent = new Date().toLocaleTimeString());
  tick();
  setInterval(tick, 1000);
}
