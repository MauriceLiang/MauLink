import type { CredentialRevealPolicy } from '../../../contracts/v1/CredentialRevealPolicy';
import type { RevealMode } from '../../../contracts/v1/RevealMode';
import { createMockIpc } from '../ipc/mock';
import { fixtureError } from './server-fixtures';

export function createCredentialRevealMock(options: { nativeAuthAvailable?: boolean } = {}) {
  let policy: CredentialRevealPolicy = {
    mode: 'deny',
    revision: 1,
    hasSecondaryPassword: false,
    nativeAuthAvailable: options.nativeAuthAvailable ?? true,
    nativeAuthReason: options.nativeAuthAvailable === false ? 'errors.nativeAuthenticationUnavailable' : null,
    lockedUntilMs: null,
  };
  const current = () => structuredClone(policy);
  const update = (mode: RevealMode, hasSecondaryPassword: boolean) => {
    policy = { ...policy, mode, hasSecondaryPassword, lockedUntilMs: null, revision: policy.revision + 1 };
    return current();
  };
  const checkRevision = (expectedRevision: number) => {
    if (expectedRevision !== policy.revision) throw fixtureError('SECURITY_POLICY_REVISION_CONFLICT', 'errors.securityPolicyRevisionConflict');
  };
  const transport = createMockIpc({
    reveal_policy_get: () => current(),
    reveal_policy_enable_protected: payload => {
      checkRevision(payload.expectedRevision);
      if (!policy.nativeAuthAvailable) throw fixtureError('NATIVE_AUTHENTICATION_UNAVAILABLE', 'errors.nativeAuthenticationUnavailable');
      if (payload.password.length < 12 || payload.password !== payload.confirmPassword) throw fixtureError('VALIDATION_FAILED', 'errors.secondaryPasswordInvalid');
      return update('protected', true);
    },
    reveal_policy_enable_direct: payload => {
      checkRevision(payload.expectedRevision);
      if (!policy.nativeAuthAvailable) throw fixtureError('NATIVE_AUTHENTICATION_UNAVAILABLE', 'errors.nativeAuthenticationUnavailable');
      if (!payload.confirmFirstRisk || !payload.confirmSecondRisk || !['允许直接查看', 'ALLOW DIRECT VIEW'].includes(payload.confirmationText)) throw fixtureError('VALIDATION_FAILED', 'errors.directRevealConfirmationRequired');
      return update('direct', false);
    },
    reveal_policy_set_deny: payload => {
      checkRevision(payload.expectedRevision);
      return update('deny', false);
    },
    reveal_policy_change_password: payload => {
      checkRevision(payload.expectedRevision);
      if (!policy.nativeAuthAvailable) throw fixtureError('NATIVE_AUTHENTICATION_UNAVAILABLE', 'errors.nativeAuthenticationUnavailable');
      if (!payload.currentPassword || payload.password.length < 12 || payload.password !== payload.confirmPassword) throw fixtureError('VALIDATION_FAILED', 'errors.secondaryPasswordInvalid');
      return update('protected', true);
    },
    reveal_policy_recover: payload => {
      checkRevision(payload.expectedRevision);
      if (!policy.nativeAuthAvailable) throw fixtureError('NATIVE_AUTHENTICATION_UNAVAILABLE', 'errors.nativeAuthenticationUnavailable');
      return update('deny', false);
    },
    credential_reveal: payload => {
      if (policy.mode === 'deny') throw fixtureError('CREDENTIAL_REVEAL_DENIED', 'errors.credentialRevealDenied');
      if (policy.mode === 'protected' && !payload.secondaryPassword) throw fixtureError('CREDENTIAL_REVEAL_PASSWORD_REQUIRED', 'errors.credentialRevealPasswordRequired');
      // Mock transport accepts any non-empty secondary password and returns an unmistakable fixture value.
      return { kind: payload.serverId === 'fixture-key-server' ? 'passphrase' : 'password', value: 'fixture-saved-credential' };
    },
  });
  return { transport, current };
}
