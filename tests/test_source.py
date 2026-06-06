from pathhunter.source import SourceMap, _expand_vars_unix, _parse_path_line


def test_source_map_first_seen_wins():
    sm = SourceMap()
    sm.insert("/usr/bin", "first")
    sm.insert("/usr/bin", "second")
    assert sm.lookup("/usr/bin") == "first"


def test_source_map_missing():
    sm = SourceMap()
    assert sm.lookup("/nonexistent") == ""


def test_expand_vars_unix_tilde():
    assert _expand_vars_unix("~", "/home/user") == "/home/user"
    assert _expand_vars_unix("~/bin", "/home/user") == "/home/user/bin"


def test_expand_vars_unix_home():
    assert _expand_vars_unix("$HOME/bin", "/home/user") == "/home/user/bin"
    assert _expand_vars_unix("${HOME}/bin", "/home/user") == "/home/user/bin"


def test_expand_vars_unix_no_expansion():
    # Other vars should pass through unchanged
    result = _expand_vars_unix("$CARGO_HOME/bin", "/home/user")
    assert "$CARGO_HOME" in result  # not expanded


def test_parse_path_line_export():
    sm = SourceMap()
    _parse_path_line(
        sm, "export PATH=/usr/bin:/usr/local/bin", "~/.bashrc", "/home/user"
    )
    assert sm.lookup("/usr/bin") == "~/.bashrc"
    assert sm.lookup("/usr/local/bin") == "~/.bashrc"


def test_parse_path_line_no_export():
    sm = SourceMap()
    _parse_path_line(sm, "PATH=/opt/bin", "~/.profile", "/home/user")
    assert sm.lookup("/opt/bin") == "~/.profile"


def test_parse_path_line_skips_dollar_path():
    sm = SourceMap()
    _parse_path_line(sm, "export PATH=/usr/bin:$PATH", "~/.bashrc", "/home/user")
    assert sm.lookup("/usr/bin") == "~/.bashrc"
    # $PATH itself should not be inserted
    assert sm.lookup("$PATH") == ""


def test_parse_path_line_skips_unresolved_dollar():
    sm = SourceMap()
    _parse_path_line(sm, "export PATH=$UNKNOWN/bin", "~/.bashrc", "/home/user")
    assert sm.lookup("$UNKNOWN/bin") == ""


def test_parse_path_line_strips_quotes():
    sm = SourceMap()
    _parse_path_line(sm, 'export PATH="/usr/bin"', "~/.zshrc", "/home/user")
    assert sm.lookup("/usr/bin") == "~/.zshrc"


def test_parse_path_line_inline_comment():
    sm = SourceMap()
    _parse_path_line(
        sm, "export PATH=/usr/bin # added by installer", "~/.bashrc", "/home/user"
    )
    assert sm.lookup("/usr/bin") == "~/.bashrc"


def test_parse_path_line_ignores_comment_lines():
    sm = SourceMap()
    _parse_path_line(sm, "# PATH=/usr/bin", "~/.bashrc", "/home/user")
    assert sm.lookup("/usr/bin") == ""


def test_build_reads_real_config(tmp_path):
    config = tmp_path / ".bashrc"
    config.write_text("export PATH=/custom/tool/bin:$PATH\n")
    sm = SourceMap()
    from pathhunter.source import _parse_shell_config

    _parse_shell_config(sm, str(config), "~/.bashrc", str(tmp_path))
    assert sm.lookup("/custom/tool/bin") == "~/.bashrc"
