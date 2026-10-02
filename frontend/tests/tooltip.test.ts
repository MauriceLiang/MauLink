import { afterEach, expect, it } from 'vitest';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import BaseIconButton from '../src/components/base/BaseIconButton.vue';
let wrapper:VueWrapper|undefined;
afterEach(()=>{wrapper?.unmount();document.body.innerHTML='';});
it('keeps a single accessible button, forwards attrs and shows its keyboard tooltip',async()=>{
  wrapper=mount(BaseIconButton,{attachTo:document.body,props:{label:'设置'},attrs:{'data-action':'settings'},slots:{default:'图标'}});const button=wrapper.get('button');
  expect(wrapper.findAll('button')).toHaveLength(1);expect(button.attributes('title')).toBeUndefined();expect(button.attributes('aria-label')).toBe('设置');expect(button.attributes('data-action')).toBe('settings');
  (button.element as HTMLButtonElement).focus();await flushPromises();expect(document.querySelector('[role=tooltip]')?.textContent).toBe('设置');
  (button.element as HTMLButtonElement).blur();await flushPromises();expect(document.querySelector('[role=tooltip]')).toBeNull();
});
