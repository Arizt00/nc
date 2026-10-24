#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

contract="docs/spec/release/v1_0_1_governance_release_contract.json"
[ -f "$contract" ] || { echo "missing governance release contract" >&2; exit 2; }

python3 - <<'PY' "$contract"
import json,sys
path=sys.argv[1]
with open(path,'r',encoding='utf-8') as f:
    c=json.load(f)

channels = c.get('release_train',{}).get('channels',[])
for req in ('stable','beta','nightly'):
    if req not in channels:
        raise SystemExit(f"missing release channel: {req}")

gov = c.get('governance',{})
if gov.get('deprecation_min_stable_releases_before_removal',0) < 2:
    raise SystemExit('deprecation window must be >= 2 stable releases')
if gov.get('lts_support_window_months',0) < 24:
    raise SystemExit('lts support window must be >= 24 months')
if gov.get('enterprise_security_sla_hours',9999) > 24:
    raise SystemExit('enterprise security sla must be <= 24h')
if gov.get('enterprise_hotfix_target_hours',9999) > 72:
    raise SystemExit('enterprise hotfix target must be <= 72h')

if not c.get('no_breaking_without_rfc', False):
    raise SystemExit('no_breaking_without_rfc must be true')

print('OK: LTS/enterprise SLA runbook policy validated')
PY

cat <<'TXT'
Runbook actions:
1. Intake: classify issue as security/critical/data-loss/standard.
2. SLA clock starts at triage acceptance.
3. Security issues: first response <=24h.
4. Critical production regression: hotfix target <=72h.
5. LTS backports only for security/critical/data-loss.
6. Publish release notes/changelog with explicit breakage timeline (if any).
TXT
