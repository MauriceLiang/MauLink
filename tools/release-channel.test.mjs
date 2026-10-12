import assert from 'node:assert/strict';
import test from 'node:test';
import {
  channelForReleaseVersion,
  classifyReleaseTransition,
  compareReleaseVersions,
  parseReleaseVersion,
} from './release-channel.mjs';

const orderedVersions = [
  '0.1.1-rc.8',
  '0.1.1-rc.9',
  '0.1.1',
  '0.1.2-rc.1',
  '0.1.2-rc.9',
  '0.1.2-rc.10',
  '0.1.2',
  '0.1.3-rc.1',
];

test('classifies only stable and rc releases', () => {
  assert.equal(channelForReleaseVersion('0.1.1'), 'stable');
  assert.equal(channelForReleaseVersion('0.1.1-rc.9'), 'preview');
  assert.equal(channelForReleaseVersion('0.1.2-rc.10'), 'preview');
});

test('orders the release matrix by SemVer precedence', () => {
  for (let index = 1; index < orderedVersions.length; index += 1) {
    assert.equal(compareReleaseVersions(orderedVersions[index - 1], orderedVersions[index]), -1);
  }
  assert.equal(classifyReleaseTransition('0.1.1-rc.8', '0.1.1-rc.9'), 'upgrade');
  assert.equal(classifyReleaseTransition('0.1.1-rc.9', '0.1.1'), 'upgrade');
  assert.equal(classifyReleaseTransition('0.1.1', '0.1.2-rc.1'), 'upgrade');
  assert.equal(classifyReleaseTransition('0.1.2-rc.1', '0.1.1'), 'downgrade');
  assert.equal(classifyReleaseTransition('0.1.2', '0.1.2-rc.2'), 'downgrade');
  assert.equal(classifyReleaseTransition('0.1.2-rc.2', '0.1.2'), 'upgrade');
  assert.equal(classifyReleaseTransition('0.1.2-rc.9', '0.1.2-rc.10'), 'upgrade');
  assert.equal(classifyReleaseTransition('0.1.2', '0.1.2'), 'sameVersion');
  assert.equal(compareReleaseVersions('0.1.3-rc.1', '0.1.2-rc.99'), 1);
});

test('rejects unsupported or non-canonical application versions', () => {
  for (const value of [
    'v0.1.1', '01.1.1', '0.1.1-rc.0', '0.1.1-rc.01', '0.1.1-beta.1',
    '0.1.1+build.1', '0.1.1-1', '0.1.1-rc.a', '',
  ]) {
    assert.equal(parseReleaseVersion(value), null, value);
    assert.throws(() => channelForReleaseVersion(value));
  }
});

test('compares numeric identifiers without JavaScript integer truncation', () => {
  assert.equal(compareReleaseVersions('0.1.1-rc.9007199254740993', '0.1.1-rc.9007199254740992'), 1);
});
