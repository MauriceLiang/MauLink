import { afterEach, describe, expect, it } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import BaseColorPicker from '../src/components/base/BaseColorPicker.vue';
import BaseSlider from '../src/components/base/BaseSlider.vue';
import TerminalAppearanceEditor from '../src/components/terminal/TerminalAppearanceEditor.vue';
import TerminalBackgroundPositionPicker from '../src/components/terminal/TerminalBackgroundPositionPicker.vue';
import TerminalThemeModeSelector from '../src/components/terminal/TerminalThemeModeSelector.vue';
import type { TerminalAppearanceSettings } from '../../contracts/v1/TerminalAppearanceSettings';
import { defaultSettings } from '../src/terminal/preferences';

const wrappers: VueWrapper[] = [];
function mounted(component: Parameters<typeof mount>[0], props: Record<string, unknown> = {}) {
  const wrapper = mount(component, { props, attachTo: document.body });
  wrappers.push(wrapper);
  return wrapper;
}
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = ''; });

function imageAppearance(): TerminalAppearanceSettings {
  return {
    themeMode: 'image',
    customColors: { ...defaultSettings.terminalCustomColors },
    backgroundImage: { ...defaultSettings.terminalBackgroundImage },
  };
}

describe('appearance controls', () => {
  it('clamps a slider value, formats its unit, and responds to keyboard changes', async () => {
    const wrapper = mounted(BaseSlider, { modelValue: 120, label: '图片不透明度', min: 10, max: 100, unit: '%' });
    await flushPromises();
    const slider = wrapper.get('[role="slider"]');
    expect(slider.attributes('aria-valuenow')).toBe('100');
    expect(wrapper.find('output').text()).toBe('100%');
    await slider.trigger('keydown', { key: 'Home' });
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual([10]);
    const endWrapper = mounted(BaseSlider, { modelValue: 10, label: '图片不透明度', min: 10, max: 100, unit: '%' });
    await flushPromises();
    await endWrapper.get('[role="slider"]').trigger('keydown', { key: 'End' });
    expect(endWrapper.emitted('update:modelValue')?.at(-1)).toEqual([100]);
  });

  it('does not change a disabled slider', async () => {
    const wrapper = mounted(BaseSlider, { modelValue: 45, label: '遮罩强度', min: 0, max: 90, disabled: true });
    await wrapper.get('[role="slider"]').trigger('keydown', { key: 'ArrowRight' });
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
  });

  it('only emits valid HEX values from the color editor', async () => {
    const wrapper = mounted(BaseColorPicker, { modelValue: '#123456', label: '文字颜色' });
    await wrapper.get('.base-color-trigger').trigger('click');
    await flushPromises();
    const hex = document.body.querySelector<HTMLInputElement>('input[aria-label="十六进制颜色值"]')!;
    hex.value = '#AA';
    hex.dispatchEvent(new Event('input', { bubbles: true }));
    await flushPromises();
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    hex.value = '#D92D20';
    hex.dispatchEvent(new Event('input', { bubbles: true }));
    await flushPromises();
    expect(wrapper.emitted('update:modelValue')).toEqual([['#D92D20']]);
  });

  it('maps every position cell to its generated enum value', async () => {
    const wrapper = mounted(TerminalBackgroundPositionPicker, { modelValue: 'center', label: '位置' });
    const expected = ['topLeft', 'top', 'topRight', 'left', 'center', 'right', 'bottomLeft', 'bottom', 'bottomRight'];
    const buttons = wrapper.findAll('button');
    expect(buttons).toHaveLength(9);
    for (const [index, button] of buttons.entries()) {
      await button.trigger('click');
      expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual([expected[index]]);
    }
  });

  it('renders the image controls in two columns and keeps overlay choices keyboard accessible', async () => {
    const modelValue = imageAppearance();
    const wrapper = mount(TerminalAppearanceEditor, { props: { modelValue, settings: defaultSettings }, attachTo: document.body });
    wrappers.push(wrapper);
    const grid = wrapper.get('.terminal-appearance-image-grid');
    const controls = grid.get('.terminal-appearance-image-controls');
    const radios = wrapper.findAll('.terminal-appearance-segmented [role="radio"]');

    expect(grid.element.children).toHaveLength(2);
    expect(grid.element.children[0]).toBe(controls.element);
    expect(grid.element.children[1]).toBe(grid.get('.terminal-position-field').element);
    expect(controls.find('.terminal-appearance-fit-field').exists()).toBe(true);
    expect(controls.find('.terminal-appearance-overlay-field').exists()).toBe(true);
    expect(grid.find('.terminal-appearance-overlay-field').exists()).toBe(true);
    expect(wrapper.find('.terminal-appearance-image-grid > .terminal-appearance-overlay-field').exists()).toBe(false);

    expect(radios).toHaveLength(2);
    expect(radios.filter(radio => radio.attributes('data-state') === 'checked')).toHaveLength(1);
    expect(radios[0]?.attributes('data-state')).toBe('checked');
    expect(wrapper.find('.terminal-appearance-segmented-radio').exists()).toBe(false);

    await wrapper.get('[aria-label="白色"]').trigger('click');
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual([{
      ...modelValue,
      backgroundImage: { ...modelValue.backgroundImage, overlayKind: 'light' },
    }]);
    await wrapper.setProps({ modelValue: { ...modelValue, backgroundImage: { ...modelValue.backgroundImage, overlayKind: 'light' } } });
    expect(wrapper.findAll('.terminal-appearance-segmented [data-state="checked"]')).toHaveLength(1);
    expect(wrapper.get('[aria-label="白色"]').attributes('data-state')).toBe('checked');

    const lightRadio = wrapper.get('[aria-label="白色"]');
    (lightRadio.element as HTMLElement).focus();
    await lightRadio.trigger('keydown', { key: 'ArrowRight' });
    await new Promise(resolve => window.setTimeout(resolve, 0));
    await flushPromises();
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual([{
      ...modelValue,
      backgroundImage: { ...modelValue.backgroundImage, overlayKind: 'dark' },
    }]);
  });

  it('exposes a keyboard-operable single-choice theme group', async () => {
    const wrapper = mounted(TerminalThemeModeSelector, { modelValue: 'followApp', label: '终端主题' });
    const radios = wrapper.findAll('[role="radio"]');
    expect(wrapper.find('[role="radiogroup"]').element).toBeTruthy();
    expect(radios).toHaveLength(5);
    radios[0]!.element.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }));
    await flushPromises();
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual(['light']);
  });
});
