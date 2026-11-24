"""A small text-only VT model, stdlib only (the driver's terminal).

It tracks what the perf harness needs: the cell grid as text, the cursor, scroll
regions, the alternate screen, DEC 2026 synchronized updates, and it answers the
queries a terminal answers (DA1, DA2, CPR, DSR, kitty keyboard flags, DECRQM,
OSC 10/11, XTWINOPS size). DECRQM is answered honestly: set/reset for the modes
the model implements (IMPLEMENTED_MODES), 0 (not recognised) for the rest. Mode 2048
(in-band resize) is implemented as a terminal does: a report `CSI 48;rows;cols;h;w t`
when the app sets it and on every resize() while it is set. It is a model, not a GPU terminal: it says nothing about
how long a real terminal takes to paint; byte volume is the proxy for that.
Colours and attributes are not tracked (the side-by-side replay is text only).
"""

from __future__ import annotations

import codecs
import re
import unicodedata

TOKEN = re.compile(
    r"(\x1b\[[0-?]*[ -/]*[@-~]"  # CSI
    r"|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)"  # OSC
    r"|\x1b[PX^_][\s\S]*?\x1b\\"  # DCS, SOS, PM, APC
    r"|\x1b[ -/]*[0-9:;<=>?@A-OQ-WYZ\\`a-z{|}~]"  # ESC sequences (not the openers [ ] P X ^ _)
    r"|[\x00-\x1a\x1c-\x1f\x7f])"  # C0 (not ESC) and DEL
)
# DEC private modes the model implements, with their power-on state (True = set).
IMPLEMENTED_MODES = {2026: False, 2048: False, 1049: False, 1047: False, 47: False, 25: True, 7: True}
CELL_H, CELL_W = 16, 8  # pixel size reported for a cell (XTWINOPS 14/16 and the 2048 report)

CSI_RE = re.compile(r"\x1b\[([<=>?]?)([0-9;:]*)([ -/]*)([@-~])")


def _width(ch: str) -> int:
    o = ord(ch)
    if o < 0x300:
        return 1
    if unicodedata.combining(ch) or unicodedata.category(ch) in ("Mn", "Me", "Cf"):
        return 0
    return 2 if unicodedata.east_asian_width(ch) in ("W", "F") else 1


class Term:
    def __init__(self, rows: int, cols: int):
        self.rows, self.cols = rows, cols
        self.main = self._blank()
        self.alt = self._blank()
        self.g = self.main
        self.in_alt = False
        self.r = self.c = 0
        self.wrap_pending = False
        self.top, self.bot = 0, rows - 1
        self.saved = (0, 0)
        self.sync = False
        self.sync_since = 0.0
        self.sync_seen = False
        self.cursor_visible = True
        self.autowrap = True
        self.modes = dict(IMPLEMENTED_MODES)
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.pending = ""
        self.out = bytearray()  # answers to queries, drained by the caller
        self.on_sync_end = None  # callback(now) when a DEC 2026 pair closes
        self.now = 0.0
        self.last_char = " "
        self.scrolled_rows = 0  # rows scrolled off the top (full-region scrolls)
        self.alt_entries = 0
        self.repaints = 0  # full-screen clears (ED 2 / ED 3)

    def _blank(self):
        return [[" "] * self.cols for _ in range(self.rows)]

    # ---- reading the screen
    def row_text(self, i: int) -> str:
        return "".join(self.g[i]).rstrip()

    def screen_rows(self) -> list[str]:
        return [self.row_text(i) for i in range(self.rows)]

    def text(self) -> str:
        return "\n".join(self.screen_rows())

    def count(self, needle: str) -> int:
        return sum(self.row_text(i).count(needle) for i in range(self.rows))

    # ---- feeding
    def feed(self, data: bytes, now: float) -> None:
        self.now = now
        s = self.pending + self.decoder.decode(data)
        self.pending = ""
        parts = TOKEN.split(s)
        # an incomplete escape sequence stays in the last text piece: hold it for the next chunk
        last = parts[-1]
        k = last.find("\x1b")
        if k >= 0:
            self.pending = last[k:]
            parts[-1] = last[:k]
        for i, p in enumerate(parts):
            if not p:
                continue
            if i % 2 == 0:
                self._text(p)
            elif p[0] == "\x1b":
                self._esc(p)
            else:
                self._c0(p)

    # ---- printable text
    def _text(self, s: str) -> None:
        g = self.g
        cols = self.cols
        for ch in s:
            w = _width(ch)
            if w == 0:
                continue
            if self.wrap_pending:
                if self.autowrap:
                    self._index_with_cr()
                self.wrap_pending = False
            if self.c + w > cols:
                if self.autowrap:
                    self._index_with_cr()
                else:
                    self.c = cols - w
            row = g[self.r]
            row[self.c] = ch
            if w == 2 and self.c + 1 < cols:
                row[self.c + 1] = ""
            self.c += w
            self.last_char = ch
            if self.c >= cols:
                self.c = cols - 1
                self.wrap_pending = True

    def _index_with_cr(self) -> None:
        self.c = 0
        self._index()

    def _index(self) -> None:
        if self.r == self.bot:
            self._scroll_up(1)
        elif self.r < self.rows - 1:
            self.r += 1

    def _rindex(self) -> None:
        if self.r == self.top:
            self._scroll_down(1)
        elif self.r > 0:
            self.r -= 1

    def _scroll_up(self, n: int) -> None:
        n = min(n, self.bot - self.top + 1)
        if self.top == 0 and self.bot == self.rows - 1 and not self.in_alt:
            self.scrolled_rows += n
        del self.g[self.top : self.top + n]
        for _ in range(n):
            self.g.insert(self.bot - n + 1, [" "] * self.cols)

    def _scroll_down(self, n: int) -> None:
        n = min(n, self.bot - self.top + 1)
        del self.g[self.bot - n + 1 : self.bot + 1]
        for _ in range(n):
            self.g.insert(self.top, [" "] * self.cols)

    # ---- C0
    def _c0(self, ch: str) -> None:
        if ch == "\n" or ch == "\x0b" or ch == "\x0c":
            self.wrap_pending = False
            self._index()
        elif ch == "\r":
            self.c = 0
            self.wrap_pending = False
        elif ch == "\b":
            self.c = max(0, self.c - 1)
            self.wrap_pending = False
        elif ch == "\t":
            self.c = min(self.cols - 1, (self.c // 8 + 1) * 8)
            self.wrap_pending = False

    # ---- ESC
    def _esc(self, p: str) -> None:
        if p.startswith("\x1b["):
            self._csi(p)
        elif p.startswith("\x1b]"):
            self._osc(p)
        elif p[1] in "PX^_":
            return
        elif p == "\x1b7":
            self.saved = (self.r, self.c)
        elif p == "\x1b8":
            self.r, self.c = self.saved
            self.wrap_pending = False
        elif p == "\x1bM":
            self._rindex()
        elif p == "\x1bD":
            self._index()
        elif p == "\x1bE":
            self._index_with_cr()
        elif p == "\x1bc":
            self.g[:] = self._blank()
            self.r = self.c = 0
            self.top, self.bot = 0, self.rows - 1

    def _osc(self, p: str) -> None:
        body = p[2:].rstrip("\x07").removesuffix("\x1b\\")
        if body.startswith("10;?"):
            self.out += b"\x1b]10;rgb:d8d8/d8d8/d8d8\x1b\\"
        elif body.startswith("11;?"):
            self.out += b"\x1b]11;rgb:1818/1818/1818\x1b\\"

    # ---- CSI
    def _csi(self, p: str) -> None:
        m = CSI_RE.fullmatch(p)
        if not m:
            return
        priv, params, inter, final = m.groups()
        a = [int(x) if x.isdigit() else 0 for x in params.replace(":", ";").split(";")] if params else []

        def arg(i: int, d: int) -> int:
            v = a[i] if i < len(a) else 0
            return v if v else d

        if priv == "?":
            if final in "hl" and not inter:
                for v in a:
                    self._dec_mode(v, final == "h")
            elif final == "u":
                self.out += b"\x1b[?0u"
            elif final == "p" and inter == "$":
                v = a[0] if a else 0
                state = 0 if v not in self.modes else (1 if self.modes[v] else 2)
                self.out += f"\x1b[?{v};{state}$y".encode()
            return
        if priv in (">", "=", "<"):
            if priv == ">" and final == "c":
                self.out += b"\x1b[>0;0;0c"
            return
        if inter:
            return
        g = self.g
        if final == "A":
            self.r = max(self.top if self.r >= self.top else 0, self.r - arg(0, 1))
            self.wrap_pending = False
        elif final == "B" or final == "e":
            self.r = min(self.bot if self.r <= self.bot else self.rows - 1, self.r + arg(0, 1))
            self.wrap_pending = False
        elif final == "C" or final == "a":
            self.c = min(self.cols - 1, self.c + arg(0, 1))
            self.wrap_pending = False
        elif final == "D":
            self.c = max(0, self.c - arg(0, 1))
            self.wrap_pending = False
        elif final == "E":
            self.r = min(self.rows - 1, self.r + arg(0, 1))
            self.c = 0
            self.wrap_pending = False
        elif final == "F":
            self.r = max(0, self.r - arg(0, 1))
            self.c = 0
            self.wrap_pending = False
        elif final == "G" or final == "`":
            self.c = min(self.cols - 1, arg(0, 1) - 1)
            self.wrap_pending = False
        elif final == "H" or final == "f":
            self.r = min(self.rows - 1, arg(0, 1) - 1)
            self.c = min(self.cols - 1, arg(1, 1) - 1)
            self.wrap_pending = False
        elif final == "d":
            self.r = min(self.rows - 1, arg(0, 1) - 1)
            self.wrap_pending = False
        elif final == "J":
            mode = a[0] if a else 0
            if mode == 0:
                self._clear_row_from(self.r, self.c)
                for i in range(self.r + 1, self.rows):
                    g[i] = [" "] * self.cols
            elif mode == 1:
                for i in range(self.r):
                    g[i] = [" "] * self.cols
                for j in range(0, min(self.c + 1, self.cols)):
                    g[self.r][j] = " "
            else:
                for i in range(self.rows):
                    g[i] = [" "] * self.cols
                self.repaints += 1
        elif final == "K":
            mode = a[0] if a else 0
            row = g[self.r]
            if mode == 0:
                self._clear_row_from(self.r, self.c)
            elif mode == 1:
                for j in range(0, min(self.c + 1, self.cols)):
                    row[j] = " "
            else:
                g[self.r] = [" "] * self.cols
        elif final == "L":
            if self.top <= self.r <= self.bot:
                n = min(arg(0, 1), self.bot - self.r + 1)
                del g[self.bot - n + 1 : self.bot + 1]
                for _ in range(n):
                    g.insert(self.r, [" "] * self.cols)
        elif final == "M":
            if self.top <= self.r <= self.bot:
                n = min(arg(0, 1), self.bot - self.r + 1)
                del g[self.r : self.r + n]
                for _ in range(n):
                    g.insert(self.bot - n + 1, [" "] * self.cols)
        elif final == "P":
            n = min(arg(0, 1), self.cols - self.c)
            row = g[self.r]
            del row[self.c : self.c + n]
            row.extend([" "] * n)
        elif final == "@":
            n = min(arg(0, 1), self.cols - self.c)
            row = g[self.r]
            row[self.c : self.c] = [" "] * n
            del row[self.cols :]
        elif final == "X":
            n = min(arg(0, 1), self.cols - self.c)
            for j in range(self.c, self.c + n):
                g[self.r][j] = " "
        elif final == "S":
            self._scroll_up(arg(0, 1))
        elif final == "T":
            self._scroll_down(arg(0, 1))
        elif final == "r":
            t, b = arg(0, 1) - 1, arg(1, self.rows) - 1
            if 0 <= t < b < self.rows:
                self.top, self.bot = t, b
            self.r = self.c = 0
            self.wrap_pending = False
        elif final == "s":
            self.saved = (self.r, self.c)
        elif final == "u":
            self.r, self.c = self.saved
        elif final == "b":
            self._text(self.last_char * arg(0, 1))
        elif final == "n":
            v = a[0] if a else 0
            if v == 6:
                self.out += f"\x1b[{self.r + 1};{self.c + 1}R".encode()
            elif v == 5:
                self.out += b"\x1b[0n"
        elif final == "c":
            if not a or a[0] == 0:
                self.out += b"\x1b[?62;22c"
        elif final == "t":
            v = a[0] if a else 0
            if v == 18:
                self.out += f"\x1b[8;{self.rows};{self.cols}t".encode()
            elif v == 14:
                self.out += f"\x1b[4;{self.rows * CELL_H};{self.cols * CELL_W}t".encode()
            elif v == 16:
                self.out += f"\x1b[6;{CELL_H};{CELL_W}t".encode()

    def _clear_row_from(self, r: int, c: int) -> None:
        row = self.g[r]
        for j in range(c, self.cols):
            row[j] = " "

    def inband_report(self) -> bytes:
        return f"\x1b[48;{self.rows};{self.cols};{self.rows * CELL_H};{self.cols * CELL_W}t".encode()

    def _dec_mode(self, v: int, on: bool) -> None:
        if v in self.modes:
            self.modes[v] = on
        if v == 2048 and on:
            self.out += self.inband_report()  # a terminal reports the size when the mode is set
        if v == 2026:
            if on:
                self.sync = True
                self.sync_since = self.now
                self.sync_seen = True
            else:
                was = self.sync
                self.sync = False
                if was and self.on_sync_end:
                    self.on_sync_end(self.now)
        elif v in (1049, 1047, 47):
            if on and not self.in_alt:
                self.in_alt = True
                self.alt_entries += 1
                self.alt = self._blank()
                self.g = self.alt
                self.saved_main = (self.r, self.c)
                self.r = self.c = 0
            elif not on and self.in_alt:
                self.in_alt = False
                self.g = self.main
                self.r, self.c = getattr(self, "saved_main", (0, 0))
            self.top, self.bot = 0, self.rows - 1
            self.wrap_pending = False
        elif v == 25:
            self.cursor_visible = on
        elif v == 7:
            self.autowrap = on

    def resize(self, rows: int, cols: int) -> None:
        for grid in (self.main, self.alt):
            for row in grid:
                row[:] = (row + [" "] * cols)[:cols]
            while len(grid) < rows:
                grid.append([" "] * cols)
            del grid[rows:]
        self.rows, self.cols = rows, cols
        self.top, self.bot = 0, rows - 1
        self.r = min(self.r, rows - 1)
        self.c = min(self.c, cols - 1)
        if self.modes[2048]:
            self.out += self.inband_report()


def _selftest() -> None:
    t = Term(40, 120)

    def ask(data: bytes) -> bytes:
        t.feed(data, 0.0)
        out = bytes(t.out)
        t.out.clear()
        return out

    assert ask(b"\x1b[?9999$p") == b"\x1b[?9999;0$y"  # unknown mode: not recognised
    assert ask(b"\x1b[?2004$p") == b"\x1b[?2004;0$y"
    assert ask(b"\x1b[?25$p") == b"\x1b[?25;1$y"
    assert ask(b"\x1b[?2048$p") == b"\x1b[?2048;2$y"  # known, reset
    assert ask(b"\x1b[?2048h") == b"\x1b[48;40;120;640;960t"  # report when set
    assert ask(b"\x1b[?2048$p") == b"\x1b[?2048;1$y"
    t.resize(30, 100)
    assert bytes(t.out) == b"\x1b[48;30;100;480;800t"  # report on every resize
    t.out.clear()
    ask(b"\x1b[?2048l")
    t.resize(20, 50)
    assert not t.out
    print("vt.py selftest ok")


if __name__ == "__main__":
    _selftest()
