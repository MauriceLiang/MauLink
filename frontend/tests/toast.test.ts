import { afterEach, expect, it, vi } from 'vitest';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { useToast } from '../src/composables/useToast';
import BaseToastViewport from '../src/components/base/BaseToastViewport.vue';
let wrapper: VueWrapper | undefined;
afterEach(() => { wrapper?.unmount(); vi.useRealTimers(); document.body.innerHTML = ''; });
it('queues messages independently and closes each after its own duration', async () => {
  vi.useFakeTimers(); const queue=useToast(); queue.success('保存成功'); queue.error('连接失败');
  wrapper=mount(BaseToastViewport,{attachTo:document.body,props:{queue}}); await flushPromises();
  expect(document.querySelectorAll('.base-toast')).toHaveLength(2);
  await vi.advanceTimersByTimeAsync(3100); await flushPromises(); expect(document.querySelectorAll('.base-toast')).toHaveLength(1); expect(document.body.textContent).toContain('连接失败');
  await vi.advanceTimersByTimeAsync(2500); await flushPromises(); expect(document.querySelectorAll('.base-toast')).toHaveLength(0); expect(queue.messages.value).toHaveLength(0);
});
it('supports manual close, optional action and pauses while the viewport is focused', async () => {
  vi.useFakeTimers(); const queue=useToast(); const run=vi.fn(); queue.info('操作可撤销',{action:{label:'撤销',run}}); queue.warning('等待');
  wrapper=mount(BaseToastViewport,{attachTo:document.body,props:{queue}}); await flushPromises();
  const viewport=document.querySelector<HTMLElement>('.base-toast-viewport')!; viewport.focus();
  await vi.advanceTimersByTimeAsync(6000); expect(queue.messages.value.filter(item=>item.open)).toHaveLength(2);
  [...document.querySelectorAll<HTMLButtonElement>('.base-toast button')].find(value=>value.textContent==='撤销')!.click(); await flushPromises(); expect(run).toHaveBeenCalledOnce();
  document.querySelector<HTMLButtonElement>('[aria-label="关闭提示"]')!.click(); await flushPromises(); expect(queue.messages.value).toHaveLength(0);
});
