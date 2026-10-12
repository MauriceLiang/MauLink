import { toMsiProductVersion } from './msi-version.mjs';

export function windowsReleaseConfig(version) {
  return {
    bundle: {
      windows: {
        allowDowngrades: true,
        wix: { version: toMsiProductVersion(version) },
      },
    },
  };
}
