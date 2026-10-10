import { afterEach, describe, expect, it } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import ServerCard from '../src/components/server/ServerCard.vue';
import { shellServers } from '../src/harness/shell-fixtures';

const wrappers: VueWrapper[] = [];
afterEach(() => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount());
  document.body.innerHTML = '';
});

describe('server home card navigation', () => {
  it('opens details from the card target and keeps the more menu as an independent sibling control', async () => {
    const wrapper = mount(ServerCard, { attachTo: document.body, props: { server: shellServers[0]!, state: 'ready' } });
    wrappers.push(wrapper);
    const card = wrapper.get('article');
    const buttons = card.findAll('button');
    expect(buttons).toHaveLength(2);
    expect(buttons[0]!.classes()).toContain('server-card-open-target');
    expect(buttons[0]!.attributes('aria-label')).toContain('已连接');
    expect(buttons[0]!.element.contains(buttons[1]!.element)).toBe(false);

    await buttons[0]!.trigger('click');
    expect(wrapper.emitted('select')).toEqual([['web-01']]);
    await buttons[1]!.trigger('click');
    await flushPromises();
    expect(document.querySelector('[role="menu"]')).not.toBeNull();
    expect(wrapper.emitted('select')).toHaveLength(1);
    const edit = document.querySelector<HTMLElement>('[data-action="edit"]')!;
    edit.click();
    await flushPromises();
    expect(wrapper.emitted('edit')).toEqual([['web-01']]);
    expect(wrapper.emitted('select')).toHaveLength(1);
  });

  it('leaves card navigation available in read-only mode while disabling edit and delete actions', async () => {
    const wrapper = mount(ServerCard, { attachTo: document.body, props: { server: shellServers[0]!, readOnly: true } });
    wrappers.push(wrapper);
    const cardButton = wrapper.get('.server-card-open-target');
    const menuButton = wrapper.get('[aria-label="更多操作 Web-01"]');
    await cardButton.trigger('click');
    await menuButton.trigger('click');
    await flushPromises();
    const items = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')];
    expect(items).toHaveLength(2);
    expect(items.every(item => item.getAttribute('aria-disabled') === 'true')).toBe(true);
    expect(wrapper.emitted('select')).toEqual([['web-01']]);
  });
});
