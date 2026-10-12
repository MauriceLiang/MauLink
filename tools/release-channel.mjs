const RELEASE_VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-rc\.([1-9]\d*))?$/;

function compareNumeric(left, right) {
  if (left.length !== right.length) return left.length < right.length ? -1 : 1;
  return left === right ? 0 : left < right ? -1 : 1;
}

export function parseReleaseVersion(value) {
  if (typeof value !== 'string') return null;
  const match = RELEASE_VERSION.exec(value);
  if (!match) return null;
  return {
    major: match[1],
    minor: match[2],
    patch: match[3],
    rc: match[4] ?? null,
  };
}

export function channelForReleaseVersion(value) {
  const parsed = parseReleaseVersion(value);
  if (!parsed) throw new Error('Unsupported MauLink release version: ' + value);
  return parsed.rc === null ? 'stable' : 'preview';
}

export function compareReleaseVersions(left, right) {
  const a = parseReleaseVersion(left);
  const b = parseReleaseVersion(right);
  if (!a || !b) throw new Error('Expected MauLink release versions');

  for (const key of ['major', 'minor', 'patch']) {
    const result = compareNumeric(a[key], b[key]);
    if (result !== 0) return result;
  }
  if (a.rc === null || b.rc === null) {
    if (a.rc === b.rc) return 0;
    return a.rc === null ? 1 : -1;
  }
  return compareNumeric(a.rc, b.rc);
}

export function classifyReleaseTransition(current, candidate) {
  const result = compareReleaseVersions(candidate, current);
  return result === 0 ? 'sameVersion' : result > 0 ? 'upgrade' : 'downgrade';
}
