import io

from pathhunter.args import Config
from pathhunter.audit import EntryState, PathEntry
from pathhunter.display import render, _truncate, _pad_right, _fmt_detail


def _render_plain(entries, only_dead=False):
    cfg = Config(no_color=True, only_dead=only_dead)
    out = io.StringIO()
    render(out, entries, cfg)
    return out.getvalue()


def test_truncate_short():
    assert _truncate("hello", 10) == "hello"


def test_truncate_long():
    result = _truncate("abcdefghij", 5)
    assert len(result) == 5
    assert result.endswith("…")


def test_pad_right():
    assert _pad_right("hi", 5) == "hi   "
    assert _pad_right("hello", 3) == "hello"  # no truncation, just no padding


def test_fmt_detail_ok():
    e = PathEntry("/usr/bin", state=EntryState.OK)
    assert _fmt_detail(e) == ""


def test_fmt_detail_duplicate():
    e = PathEntry("/usr/bin", state=EntryState.DUPLICATE, dup_index=1)
    assert _fmt_detail(e) == "  [duplicate #1]"


def test_fmt_detail_symlink():
    e = PathEntry("/usr/bin", state=EntryState.SYMLINK, symlink_target="/real/bin")
    assert "→" in _fmt_detail(e)
    assert "/real/bin" in _fmt_detail(e)


def test_fmt_detail_dangling():
    assert "dangling" in _fmt_detail(PathEntry("/x", state=EntryState.DANGLING))


def test_fmt_detail_empty():
    assert "current directory" in _fmt_detail(
        PathEntry("(empty entry)", state=EntryState.EMPTY)
    )


def test_render_plain_header():
    entries = [PathEntry("/usr/bin", state=EntryState.OK)]
    output = _render_plain(entries)
    assert "Path Hunter" in output
    assert "1 entries" in output


def test_render_plain_summary_always_shows_dead():
    entries = [PathEntry("/usr/bin", state=EntryState.OK)]
    output = _render_plain(entries)
    assert "0 dead" in output


def test_render_plain_summary_shows_ok_when_nonzero():
    entries = [PathEntry("/usr/bin", state=EntryState.OK)]
    output = _render_plain(entries)
    assert "1 ok" in output


def test_render_only_dead_filters():
    entries = [
        PathEntry("/usr/bin", state=EntryState.OK),
        PathEntry("/dead/bin", state=EntryState.DEAD),
    ]
    output = _render_plain(entries, only_dead=True)
    assert "/dead/bin" in output
    assert "/usr/bin" not in output


def test_render_colored_no_crash():
    entries = [
        PathEntry("/usr/bin", state=EntryState.OK),
        PathEntry("/bad/path", state=EntryState.DEAD),
        PathEntry("/usr/bin", state=EntryState.DUPLICATE, dup_index=1),
    ]
    cfg = Config(no_color=False)
    out = io.StringIO()
    render(out, entries, cfg)
    text = out.getvalue()
    assert "Path Hunter" in text
    assert "\x1b[" in text  # ANSI codes present
