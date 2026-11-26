# Host effect fixtures

Run `python3 -I tools/rtwk-port/host-effects-layout/rewrite-fixtures.py` to migrate the
listed fixture patterns from dedicated host commands to their typed payload inside
`Command::Effect`. The script is idempotent. Run the port formatter afterward.
