// jsdom lacks layout observers and pointer capture; behavior is also checked in the browser.
class TestResizeObserver { observe() {} unobserve() {} disconnect() {} }
Object.defineProperty(globalThis, 'ResizeObserver', { value: TestResizeObserver, configurable: true });
Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { value() {}, configurable: true });
Object.defineProperty(HTMLElement.prototype, 'hasPointerCapture', { value() { return false; }, configurable: true });
Object.defineProperty(HTMLElement.prototype, 'setPointerCapture', { value() {}, configurable: true });
Object.defineProperty(HTMLElement.prototype, 'releasePointerCapture', { value() {}, configurable: true });
