import assert from 'node:assert/strict';
import test from 'node:test';
import { toMsiProductVersion as convert } from './msi-version.mjs';

const cases = [
  ['0.1.0', '0.1.999'],
  ['v0.1.1-rc.1', '0.1.1001'],
  ['0.1.1-rc.5', '0.1.1005'],
  ['v0.1.1-rc.6', '0.1.1006'],
  ['0.1.1-rc.7', '0.1.1007'],
  ['0.1.1', '0.1.1999'],
  ['0.1.2-rc.1', '0.1.2001'],
  ['1.0.0-rc.1', '1.0.1'],
  ['255.255.64', '255.255.64999'],
];

for (const [source, expected] of cases) {
  test('map ' + source + ' => ' + expected, () => assert.equal(convert(source), expected));
}

for (const input of [
  '0.1.1-rc.0',
  '0.1.1-rc.999',
  '0.1.65',
  '256.1.1',
  '1.256.1',
  '1.1.1-beta.1',
  '01.1.1',
  '1.1.1+build.1',
  '1.1.1-rc.a',
  '1.1.1.7',
  '',
  'abc',
]) {
  test('reject ' + JSON.stringify(input), () => assert.throws(() => convert(input)));
}

test('MSI three-field order matches RC -> GA -> next RC', () => {
  const sequence = [
    '0.1.0',
    '0.1.1-rc.1',
    '0.1.1-rc.6',
    '0.1.1-rc.7',
    '0.1.1',
    '0.1.2-rc.1',
  ];
  const versions = sequence.map(convert).map(version => version.split('.').map(Number));

  for (let index = 1; index < versions.length; index += 1) {
    assert.ok(
      compare(versions[index - 1], versions[index]) < 0,
      sequence[index - 1] + ' must be earlier than ' + sequence[index],
    );
  }
});

function compare(left, right) {
  for (let index = 0; index < 3; index += 1) {
    if (left[index] !== right[index]) return left[index] - right[index];
  }
  return 0;
}
