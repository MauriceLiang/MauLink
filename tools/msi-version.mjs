const VERSION_PATTERN =
  /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-rc\.(0|[1-9]\d*))?$/;

/**
 * Map supported MauLink SemVer releases to the three-field MSI ProductVersion.
 * Windows Installer compares only these three fields.
 */
export function toMsiProductVersion(input) {
  if (typeof input !== 'string') {
    throw new Error('Expected a version string');
  }

  const match = VERSION_PATTERN.exec(input);
  if (!match) {
    throw new Error('Unsupported MauLink MSI release version: ' + input);
  }

  const major = Number(match[1]);
  const minor = Number(match[2]);
  const patch = Number(match[3]);
  const rc = match[4] === undefined ? null : Number(match[4]);

  if (!Number.isSafeInteger(major) || major > 255 || !Number.isSafeInteger(minor) || minor > 255) {
    throw new Error('MSI major/minor must be within 0..255');
  }
  if (!Number.isSafeInteger(patch) || patch > 64) {
    throw new Error('Supported patch range is 0..64; revise mapping before publishing');
  }
  if (rc !== null && (!Number.isSafeInteger(rc) || rc < 1 || rc > 998)) {
    throw new Error('Supported release candidate number is 1..998');
  }

  const build = patch * 1000 + (rc ?? 999);
  if (build > 65535) {
    throw new Error('MSI build field exceeds 65535');
  }

  return [major, minor, build].join('.');
}
