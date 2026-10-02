import { afterEach, expect, it } from 'vitest';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import BaseSelect from '../src/components/base/BaseSelect.vue';
const wrappers: VueWrapper[] = [];
afterEach(() => { wrappers.splice(0).forEach(value => value.unmount()); document.body.innerHTML = ''; });
const key = (element: Element, value: string) => element.dispatchEvent(new KeyboardEvent('keydown', { key:value, bubbles:true, cancelable:true }));
it('supports placeholder, disabled skipping, keyboard selection, Escape and focus restore', async () => {
  const wrapper = mount(BaseSelect, { attachTo:document.body, props:{ label:'选择', placeholder:'请选择', options:[{value:'off',label:'禁用',disabled:true},{value:'one',label:'第一项'},{value:'two',label:'第二项'}] } }); wrappers.push(wrapper);
  const trigger = wrapper.get('[role=combobox]');
  expect(trigger.text()).toContain('请选择');
  (trigger.element as HTMLElement).focus(); key(trigger.element, 'ArrowDown'); await flushPromises();
  const options = [...document.querySelectorAll<HTMLElement>('[role=option]')];
  expect(document.activeElement).toBe(options[1]); expect(options[0]?.hasAttribute('data-disabled')).toBe(true);
  key(document.activeElement!, 'ArrowDown'); await new Promise(resolve => setTimeout(resolve, 0)); await flushPromises(); expect(document.activeElement).toBe(options[2]);
  key(document.activeElement!, 'Enter'); await flushPromises(); expect(wrapper.emitted('update:modelValue')).toEqual([['two']]);
  await new Promise(resolve => setTimeout(resolve, 0)); expect(document.activeElement).toBe(trigger.element);
  key(trigger.element, 'ArrowDown'); await flushPromises(); key(document.activeElement!, 'Escape'); await flushPromises();
  expect(document.querySelector('[role=listbox]')).toBeNull();
});
it('preserves nullable values and never opens a disabled select', async () => {
  const wrapper=mount(BaseSelect,{attachTo:document.body,props:{label:'代理',modelValue:'one',options:[{value:null,label:'无'},{value:'one',label:'一'}]}}); wrappers.push(wrapper);
  key(wrapper.get('[role=combobox]').element,'ArrowDown'); await flushPromises();
  key(document.querySelectorAll('[role=option]')[0]!, 'Enter'); await flushPromises(); expect(wrapper.emitted('update:modelValue')).toEqual([[null]]);
  await wrapper.setProps({disabled:true}); key(wrapper.get('[role=combobox]').element,'ArrowDown'); await flushPromises(); expect(document.querySelector('[role=listbox]')).toBeNull(); expect(wrapper.get('button').attributes('disabled')).toBeDefined();
});
