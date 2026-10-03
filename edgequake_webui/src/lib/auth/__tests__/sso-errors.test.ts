import { describe, expect, it } from "vitest";
import {
  SSO_ERROR_FALLBACK_KEY,
  SSO_ERROR_KEYS,
  ambiguousOrgAliases,
  ssoErrorBase,
  ssoErrorKey,
} from "../sso-errors";

import en from "@/locales/en.json";
import fr from "@/locales/fr.json";
import zh from "@/locales/zh.json";

function lookup(dict: Record<string, unknown>, key: string): unknown {
  return key.split(".").reduce<unknown>((acc, part) => (acc as Record<string, unknown> | undefined)?.[part], dict);
}

describe("SPEC-158 sso-errors", () => {
  it("maps every backend denial code to a translated message in en/fr/zh", () => {
    for (const key of [...Object.values(SSO_ERROR_KEYS), SSO_ERROR_FALLBACK_KEY]) {
      for (const [name, dict] of [["en", en], ["fr", fr], ["zh", zh]] as const) {
        expect(typeof lookup(dict as Record<string, unknown>, key), `${name}:${key}`).toBe("string");
      }
    }
  });

  it("covers the documented denial and conflict codes", () => {
    for (const code of [
      "org_unknown", "org_missing", "org_ambiguous", "tenant_suspended", "hd_mismatch",
      "idp_tenant_not_allowed", "jit_disabled", "max_users", "membership_revoked",
      "tenant_access_denied", "account_exists_unlinked", "sso_unavailable",
    ]) {
      expect(SSO_ERROR_KEYS[code], code).toBeDefined();
    }
  });

  it("ignores the :detail suffix and never renders unknown codes raw", () => {
    expect(ssoErrorBase("org_ambiguous:acme,globex")).toBe("org_ambiguous");
    expect(ssoErrorKey("org_ambiguous:acme,globex")).toBe(SSO_ERROR_KEYS.org_ambiguous);
    expect(ssoErrorKey("totally_new_code")).toBe(SSO_ERROR_FALLBACK_KEY);
    expect(ssoErrorKey(null)).toBe(SSO_ERROR_FALLBACK_KEY);
  });

  it("extracts picker aliases only from org_ambiguous", () => {
    expect(ambiguousOrgAliases("org_ambiguous:globex, acme")).toEqual(["globex", "acme"]);
    expect(ambiguousOrgAliases("org_unknown")).toEqual([]);
    expect(ambiguousOrgAliases(undefined)).toEqual([]);
  });
});
