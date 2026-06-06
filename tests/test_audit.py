import sys

import pytest

from pathhunter.audit import EntryState, PathEntry, audit_scan, path_entry_is_dead


class StubSourceMap:
    def lookup(self, key: str) -> str:
        return ""

    def is_user(self, key: str) -> bool:
        return False


def test_ok_entry(tmp_path):
    sm = StubSourceMap()
    entries = audit_scan(sm, str(tmp_path))
    assert len(entries) == 1
    assert entries[0].state == EntryState.OK


def test_dead_entry():
    sm = StubSourceMap()
    entries = audit_scan(sm, "/this/path/does/not/exist/ever")
    assert entries[0].state == EntryState.DEAD


def test_empty_entry():
    sm = StubSourceMap()
    sep = ";" if sys.platform == "win32" else ":"
    entries = audit_scan(sm, f"/usr/bin{sep}{sep}/usr/local/bin")
    # Middle entry is empty
    assert entries[1].state == EntryState.EMPTY
    assert entries[1].path == "(empty entry)"


def test_duplicate_entry(tmp_path):
    sm = StubSourceMap()
    sep = ";" if sys.platform == "win32" else ":"
    raw = f"{tmp_path}{sep}{tmp_path}"
    entries = audit_scan(sm, raw)
    assert entries[0].state == EntryState.OK
    assert entries[1].state == EntryState.DUPLICATE
    assert entries[1].dup_index == 1


def test_third_occurrence_is_dup_index_2(tmp_path):
    sm = StubSourceMap()
    sep = ";" if sys.platform == "win32" else ":"
    raw = f"{tmp_path}{sep}{tmp_path}{sep}{tmp_path}"
    entries = audit_scan(sm, raw)
    assert entries[2].dup_index == 2


def test_file_entry(tmp_path):
    f = tmp_path / "somefile.txt"
    f.write_text("hi")
    sm = StubSourceMap()
    entries = audit_scan(sm, str(f))
    assert entries[0].state == EntryState.FILE


@pytest.mark.skipif(
    sys.platform == "win32", reason="symlinks behave differently on Windows"
)
def test_symlink_entry(tmp_path):
    target = tmp_path / "real"
    target.mkdir()
    link = tmp_path / "link"
    link.symlink_to(target)
    sm = StubSourceMap()
    entries = audit_scan(sm, str(link))
    assert entries[0].state == EntryState.SYMLINK
    assert entries[0].symlink_target is not None


@pytest.mark.skipif(
    sys.platform == "win32", reason="symlinks behave differently on Windows"
)
def test_dangling_symlink(tmp_path):
    link = tmp_path / "dangling"
    link.symlink_to(tmp_path / "nowhere")
    sm = StubSourceMap()
    entries = audit_scan(sm, str(link))
    assert entries[0].state == EntryState.DANGLING


def test_path_entry_is_dead():
    assert path_entry_is_dead(PathEntry("/x", state=EntryState.DEAD))
    assert path_entry_is_dead(PathEntry("/x", state=EntryState.DANGLING))
    assert path_entry_is_dead(PathEntry("/x", state=EntryState.FILE))
    assert not path_entry_is_dead(PathEntry("/x", state=EntryState.OK))
    assert not path_entry_is_dead(PathEntry("/x", state=EntryState.SYMLINK))
    assert not path_entry_is_dead(PathEntry("/x", state=EntryState.DUPLICATE))
