import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path

root = Path(__file__).resolve().parent
log_path = root / 'etalon-full-resume-20261116-161454.log'
text = log_path.read_text(encoding='utf-8-sig')
rows = []
for line in text.splitlines():
    match = re.match(r'\s*(PASS|FAIL|ABORT|TIMEOUT)\s+\[([^]]+)\]\s+\(\s*(\d+)/5507\)\s+(.*)', line)
    if match:
        rows.append(dict(status=match[1], elapsed=match[2].strip(), ordinal=int(match[3]), test=match[4]))
report = {
    'schema': 'webizen.interrupted-test-run/v1',
    'status': 'INTERRUPTED',
    'reason': 'Parent explicitly requested stopping an already-failing full run to investigate two concrete failures; full suite still required after fixes.',
    'revision': '785ab9c1b7d729db9afcd9b0f3fd70344b37be1e',
    'command': 'cargo xtask test',
    'nextest_run_id': '74894922-5f99-4b7a-a89e-9f15eb6d98d8',
    'build': 'passed; all-feature test profile completed in 16m50s',
    'gpu_discovery': 'passed; one test in 3.234s',
    'scheduled_tests': 5507,
    'configured_skipped': 54,
    'default_filter_skipped_subset': 36,
    'completed_cases': len(rows),
    'case_counts': {status: sum(r['status'] == status for r in rows) for status in ['PASS','FAIL','ABORT','TIMEOUT']},
    'remaining_not_completed': 5507-len(rows),
    'failures': [r for r in rows if r['status'] != 'PASS'],
    'qualification': 'PASS includes harness cases labelled Unsupported and Executed Failure. Do not equate every passing harness case with a successful GPU capability execution.',
    'interruption': 'Ctrl+C on owned session92114; exited1; subsequent process inventory found no process beneath target-webizen-tests.',
    'no_reproduction_started': True,
    'log': log_path.name,
    'log_sha256': hashlib.sha256(log_path.read_bytes()).hexdigest(),
    'recorded_utc': datetime.now(timezone.utc).isoformat(),
}
(root/'etalon-full-interrupted.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
(root/'etalon-full-completed-cases.json').write_text(json.dumps(rows, indent=2)+'\n', encoding='utf8')
print(json.dumps(report, indent=2))
