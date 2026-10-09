import { afterEach, expect, it, vi } from 'vitest';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { useToast } from '../src/composables/useToast';
import BaseToastViewport from '../src/components/base/BaseToastViewport.vue';
import { locale } from '../src/i18n/locale';
let wrapper: VueWrapper | undefined;
afterEach(() => { wrapper?.unmount(); vi.useRealTimers(); locale.value = 'zh-CN'; document.body.innerHTML = ''; });
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

it('shows localized two-line titles and renders notification content as text', async () => {
  const queue=useToast(); const successId=queue.success('<script>alert(1)</script>'); queue.info('提示内容'); queue.warning('请注意'); queue.error('连接失败');
  wrapper=mount(BaseToastViewport,{attachTo:document.body,props:{queue}}); await flushPromises();
  expect([...document.querySelectorAll('.base-toast-title')].map(element=>element.textContent)).toEqual(['操作成功','提示','需要注意']);
  expect(document.querySelector('script')).toBeNull(); expect(document.querySelector('.base-toast-description')?.textContent).toBe('<script>alert(1)</script>');
  queue.remove(successId); await flushPromises();
  expect([...document.querySelectorAll('.base-toast-title')].map(element=>element.textContent)).toEqual(['提示','需要注意','操作失败']);
  locale.value='en'; await flushPromises();
  expect([...document.querySelectorAll('.base-toast-title')].map(element=>element.textContent)).toEqual(['Notice','Warning','Error']);
  expect(document.querySelector('[aria-label="Close notification"]')).not.toBeNull();
});

it('uses a single viewport with all six saved-position anchors', async () => {
  const queue=useToast(); wrapper=mount(BaseToastViewport,{attachTo:document.body,props:{queue}});
  const viewport=document.querySelector('.base-toast-viewport')!;
  expect(viewport.getAttribute('data-position')).toBe('topRight');
  for (const position of ['topLeft','topCenter','topRight','bottomLeft','bottomCenter','bottomRight']) {
    await wrapper.setProps({position}); expect(viewport.getAttribute('data-position')).toBe(position);
  }
});

it('deduplicates identical ordinary messages for two seconds but preserves actions', () => {
  vi.useFakeTimers(); const queue=useToast();
  const first=queue.success('保存成功'); expect(queue.success('保存成功')).toBe(first); expect(queue.messages.value).toHaveLength(1);
  vi.advanceTimersByTime(2001); expect(queue.success('保存成功')).not.toBe(first); expect(queue.messages.value).toHaveLength(2);
  const action={label:'撤销',run:vi.fn()}; const actionOne=queue.info('可撤销',{action}); const actionTwo=queue.info('可撤销',{action});
  expect(actionTwo).not.toBe(actionOne);
});

it('shows at most three messages and promotes pending messages in FIFO order', () => {
  const queue=useToast();
  const ids=[queue.success('第一条'),queue.info('第二条'),queue.warning('第三条'),queue.error('第四条'),queue.info('第五条')];
  expect(queue.messages.value.map(item=>item.id)).toEqual(ids.slice(0,3));
  queue.remove(ids[0]); expect(queue.messages.value.map(item=>item.id)).toEqual([ids[1],ids[2],ids[3]]);
  queue.remove(ids[2]); expect(queue.messages.value.map(item=>item.id)).toEqual([ids[1],ids[3],ids[4]]);
});

it('starts a pending message duration only after it becomes visible', async () => {
  vi.useFakeTimers(); const queue=useToast(); wrapper=mount(BaseToastViewport,{attachTo:document.body,props:{queue}});
  queue.success('第一条'); queue.info('第二条'); queue.warning('第三条'); const pendingId=queue.error('等待中的错误');
  await flushPromises(); expect(document.querySelectorAll('.base-toast')).toHaveLength(3);
  await vi.advanceTimersByTimeAsync(6000); await flushPromises();
  expect(queue.messages.value).toHaveLength(1); expect(queue.messages.value[0]?.id).toBe(pendingId); expect(queue.messages.value[0]?.open).toBe(true);
});
