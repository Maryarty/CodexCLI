"""Helpers of the oracle-guard rewrites: exact substitutions in shard files."""
import os
import re

ORACLE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..',
                      'codex-rs', 'tui-rtwk', 'tests', 'oracle')


def path(name):
    return os.path.join(ORACLE, name + '.rs')


class Shard:
    def __init__(self, name):
        self.name = name
        self.text = open(path(name)).read()

    def sub(self, old, new, count=1):
        n = self.text.count(old)
        if n == 0:
            # already rewritten by an earlier run
            if self.text.count(new) >= 1 or new == '':
                return self
            raise SystemExit('%s: not found: %r' % (self.name, old[:80]))
        if n != count:
            raise SystemExit('%s: %d matches (want %d): %r' % (self.name, n, count, old[:80]))
        self.text = self.text.replace(old, new)
        return self

    def rx(self, pattern, repl, count=None, flags=re.S):
        new, n = re.subn(pattern, repl, self.text, flags=flags)
        if n == 0:
            print('%s: no match (done before?): %r' % (self.name, pattern[:60]))
            return self
        if count is not None and n != count:
            raise SystemExit('%s: %d matches (want %d): %r' % (self.name, n, count, pattern[:80]))
        self.text = new
        return self

    def save(self):
        open(path(self.name), 'w').write(self.text)
