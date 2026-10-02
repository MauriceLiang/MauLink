import { afterEach, expect, it } from 'vitest';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import BaseCheckbox from '../src/components/base/BaseCheckbox.vue';
import BaseSwitch from '../src/components/base/BaseSwitch.vue';
import BaseCollapsible from '../src/components/base/BaseCollapsible.vue';
const wrappers: VueWrapper[]=[];
afterEach(()=>{wrappers.splice(0).forEach(wrapper=>wrapper.unmount());document.body.innerHTML='';});
for (const component of [BaseCheckbox,BaseSwitch]) it(`${component.__name} associates its label, emits edits and blocks disabled actions`,async()=>{
  const wrapper=mount(component,{attachTo:document.body,props:{modelValue:false,label:'选项'}});wrappers.push(wrapper);
  const button=wrapper.get('button');expect(wrapper.get('label').attributes('for')).toBe(button.attributes('id'));expect(button.attributes('aria-checked')).toBe('false');
  await button.trigger('click');expect(wrapper.emitted('update:modelValue')).toEqual([[true]]);await wrapper.setProps({modelValue:true,disabled:true});expect(button.attributes('aria-checked')).toBe('true');(button.element as HTMLButtonElement).click();expect(wrapper.emitted('update:modelValue')).toHaveLength(1);
});
it('announces expandable content and prevents a disabled toggle',async()=>{
  const wrapper=mount(BaseCollapsible,{props:{open:false,label:'高级'},slots:{default:'高级选项'}});wrappers.push(wrapper);const button=wrapper.get('button');expect(button.attributes('aria-expanded')).toBe('false');await button.trigger('click');expect(wrapper.emitted('update:open')).toEqual([[true]]);await wrapper.setProps({open:true,disabled:true});await flushPromises();expect(button.attributes('aria-expanded')).toBe('true');expect(wrapper.text()).toContain('高级选项');(button.element as HTMLButtonElement).click();expect(wrapper.emitted('update:open')).toHaveLength(1);
});
