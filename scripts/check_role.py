#!/usr/bin/env python3
from pathlib import Path
import sys
text = Path('.ai-context.yml').read_text(encoding='utf-8')
need = [
    '  role: learning_proposal_engine',
    'role_contract:',
    '  name: learning_proposal_engine',
    '  authority: retrospective_analysis_only',
    '  unavailable_effect: existing_rules_remain_static',
    '    - no_task_ownership',
    '    - no_ledger_writes',
    '    - no_silent_weight_changes',
]
for item in need:
    if item not in text:
        print(f'role-contract: missing {item!r}', file=sys.stderr)
        raise SystemExit(1)
print('role-contract: OK heimlern')
