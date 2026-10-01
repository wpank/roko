# PreToolUse guard for Claude CLI agents.
#
# build_settings_json (claude_cli_agent.rs) writes this script into the
# Claude CLI `--settings` payload, which runs it with `python3 -c` before a
# tool call and passes the hook input as JSON on stdin. The first argument
# names the check.
#
# `bash` (Bash calls) denies commands that name a provider key file and
# commands that discard the operator's work or move branches. A command
# names a key file when, in its text or its words with quotes removed
# (those of sh -c '...' strings too):
#
# - a path to one appears (.roko/.env, ~/.roko/config.toml), or a glob
#   directly in a .roko directory (.roko/*);
# - a word names a .roko directory itself (cd ~/.roko) and another ends in
#   a key file's name (.env) or is a bare glob (*);
# - a word, with its braces and glob expanded (cat roko.{toml,lock}, cat *)
#   and resolved with symlinks followed against the working directory and
#   each cd target in the command, is a key file, or a roko config file that
#   holds a secret (see below). A brace expansion that makes more words than
#   the guard checks (BRACE_LIMIT) is denied.
#
# The destructive commands are:
#
# - git checkout, switch, restore and push;
# - git branch -m, -M and -D (renames and force deletes);
# - git reset --hard;
# - git stash, except stash list and stash show;
# - git clean, except dry runs (-n);
# - recursive rm (-r, -R or --recursive, anywhere before --);
# - find -delete, and any rm run on what find or fd lists: by find -exec,
#   fd -x, a pipe (find . | xargs rm, find . | while read f; do rm ...) or a
#   substitution (rm $(find ...));
# - rm, unlink or shred run by xargs or parallel, whatever feeds them
#   (ls | xargs rm, xargs rm < list, xargs -a list rm, parallel rm ::: a b).
#
# Every command in a chain is checked (;, &&, ||, |, & and newlines), after
# assignments, shell keywords and wrappers such as sudo, env and xargs, and
# past git's global options (-C, -c, --git-dir, --work-tree), after which
# comes git's subcommand (sudo git -C dir rm -r x runs git rm). Commands run
# by subshells, sh -c, eval, $(...), backquotes, find -exec, fd -x, busybox
# applets, ssh (remotely) and command strings handed to wrappers (watch
# '...', flock -c '...', parallel '...') are checked too.
#
# A git subcommand that is not one of git's own commands is looked up as an
# alias, where the Bash call runs and with the command's -C and -c options,
# and the alias is checked in its place (a ! alias as a shell command). A
# subcommand that is neither is denied, since it may be an alias the same
# command defines, and so is an alias in a command that changes directory
# or git's environment, where the alias may mean something else.
#
# `file` (Read, Edit, Write, Grep, Glob and the like) denies a file_path,
# notebook_path or path argument that is a provider key file: .env,
# secrets.toml, credentials.json or config.toml in any .roko directory,
# ~/.roko included. A Grep or Glob rooted at a .roko directory is denied as
# well. The rest of .roko stays readable: when HOME is the workdir, ~/.roko
# is the workdir's .roko, with its plans, state and plan worktrees.
#
# A roko config file outside .roko (roko.toml, the file ROKO_CONFIG names,
# the legacy ~/.config/roko/config.toml) is denied while it holds a secret
# such as serve.auth.api_key, and so is a Grep of a tree that holds one,
# unless its glob or type leaves the file out. roko itself refuses to load
# such a file (the secret belongs in .roko/.env), so this matters for a
# secret added while roko runs.
#
# A Bash command that reads a whole tree is denied when the tree holds a key
# file or such a config and the read reaches it: a recursive search (grep
# -r, rg, ag, ack, and git grep, which reads untracked files, key files
# among them, only with --untracked or --no-index), or a read (cat, grep,
# cp) of what find, fd or xargs lists. A tree holds the key files in its
# .roko, in that of each subdirectory two levels down, and in the
# workspace's (in the directory the command or the call runs in, or above
# it) and ~/.roko when it holds those. A read reaches a file unless its
# filters leave the file out (grep --exclude-dir=.roko, rg -g '!.roko', a
# find test or -prune, an fd pattern) or it skips hidden files, as rg, ag
# and fd do without --hidden, or hidden directories, as a list from ls does.
# A command after a cd is judged where it runs; a script or a variable can
# still hide the read. roko-std's bash tool applies the same rules
# (sandbox/reads.rs), and both test the commands in roko-std's
# sandbox/secret_read_cases.txt.
#
# Exit 0 lets the call run. Exit 2 blocks it, and Claude Code shows stderr
# to the model. Claude Code treats any other exit code as a non-blocking
# error and runs the call anyway, so every failure here, including
# unreadable input, exits 2.
#
# This is best effort, not a sandbox: a script file or a variable can still
# hide a command or a path from it.

import contextlib
import fnmatch
import functools
import glob
import json
import os
import re
import shlex
import stat
import subprocess
import sys


def block(reason):
    sys.stderr.write("BLOCKED: " + reason + "\n")
    sys.exit(2)


# Words that can start a command without being its program.
SHELL_KEYWORDS = {"if", "then", "else", "elif", "do", "while", "until", "!", "{", "}", "time"}
# Programs that run another program named later in their arguments.
WRAPPERS = {
    "sudo", "doas", "env", "command", "builtin", "exec", "nohup", "nice", "ionice", "time",
    "timeout", "gtimeout", "xargs", "stdbuf", "unbuffer", "chronic", "caffeinate", "watch", "flock",
    "su", "runuser", "script", "sg", "parallel",
}
# Wrappers that hand every argument to sh -c as one command (watch 'rm -rf x').
STRING_WRAPPERS = {"watch", "sg", "parallel"}
# Wrapper options whose value is a shell command (flock -c, su -c, env -S).
COMMAND_OPTIONS = {"-c", "--command", "-S", "--split-string"}
# Wrapper options whose value is a user or group, never the program (sudo -u git).
USER_OPTIONS = {"-u", "-g", "-U", "--user", "--group", "--other-user"}
# Wrapper options whose value is the next word (sudo -u git, xargs -a list),
# so a git there is no program and the word after it no git subcommand.
VALUE_OPTIONS = USER_OPTIONS | {
    "-a", "--arg-file", "-d", "--delimiter", "-E", "-I", "-L", "-n", "--max-args", "-P",
    "--max-procs", "-s", "--max-chars", "-C", "--chdir", "-D", "-h", "-p", "-r", "-t", "-T",
    "-k", "--kill-after", "--signal", "-w", "--timeout",
}
# Multi-call binaries whose first argument names the program (busybox rm).
MULTICALL = {"busybox", "toybox"}
# Programs that list a tree's files; find's actions and fd's options that run
# a command on each one (find -exec, fd -x).
FINDERS = {"find", "fd", "fdfind"}
FIND_EXEC = {"-exec", "-execdir", "-ok", "-okdir"}
FD_EXEC = {"-x", "--exec", "-X", "--exec-batch"}
# Programs that search file contents: grep and its variants, which recurse
# with -r (rgrep always does), and ripgrep, which always does. The options
# of each that take a value, given in the same word or the next.
GREPS = {"grep", "egrep", "fgrep", "rgrep"}
RIPGREPS = {"rg"}
GREP_SHORT_VALUES = set("efmABCdD")
GREP_LONG_VALUES = {
    "--regexp", "--file", "--max-count", "--after-context", "--before-context", "--context",
    "--directories", "--devices", "--include", "--exclude", "--exclude-dir", "--exclude-from",
    "--label", "--binary-files", "--group-separator",
}
RG_SHORT_VALUES = set("efgtTmABCjMdEr")
RG_LONG_VALUES = {
    "--regexp", "--file", "--glob", "--iglob", "--type", "--type-not", "--type-add",
    "--type-clear", "--max-count", "--after-context", "--before-context", "--context",
    "--threads", "--max-columns", "--max-depth", "--maxdepth", "--encoding", "--engine", "--sort",
    "--sortr", "--pre", "--pre-glob", "--color", "--colors", "--context-separator",
    "--field-context-separator", "--field-match-separator", "--path-separator", "--replace",
    "--max-filesize", "--dfa-size-limit", "--regex-size-limit", "--ignore-file",
    "--hostname-bin", "--hyperlink-format",
}
AG_LONG_VALUES = {
    "--after", "--before", "--context", "--file-search-regex", "--max-count", "--path-to-ignore",
    "--depth", "--ignore", "--ignore-dir", "--pager", "--width",
}
ACK_LONG_VALUES = {
    "--after-context", "--before-context", "--context", "--max-count", "--type", "--ignore-dir",
    "--ignore-file", "--match", "--output", "--pager",
}
GIT_GREP_LONG_VALUES = {
    "--max-count", "--after-context", "--before-context", "--context", "--max-depth", "--threads",
}
# Per searcher: its options that take a value (short letters, long names),
# those with which it lists file names and reads none, and those that give
# the pattern, so that no operand is.
SEARCH_OPTIONS = {
    "grep": (GREP_SHORT_VALUES, GREP_LONG_VALUES, set(), {"-e", "-f", "--regexp", "--file"}),
    "rg": (
        RG_SHORT_VALUES, RG_LONG_VALUES, {"--files", "--type-list"},
        {"-e", "-f", "--regexp", "--file"},
    ),
    "ag": (set("ABCGgmpW"), AG_LONG_VALUES, {"-g", "--list-file-types"}, set()),
    "ack": (set("ABCgmt"), ACK_LONG_VALUES, {"-g", "-f", "--help-types"}, {"--match"}),
}
SEARCHERS = GREPS | RIPGREPS | {"ag", "ack"}
# rg's options that give a glob (! leaves files out).
RG_GLOBS = {"-g", "--glob", "--iglob"}
# Programs that print or copy the files they are given, and so read a secret
# when a list the guard cannot see (find -exec, xargs) names one.
READERS = SEARCHERS | {
    "cat", "tac", "head", "tail", "less", "more", "nl", "sed", "awk", "gawk", "cut", "sort",
    "uniq", "strings", "od", "xxd", "hexdump", "base64", "diff", "paste", "jq", "yq", "cp",
    "rsync", "tar", "zip",
}
# find's tests on a file's name or path; its other primaries that take one
# argument (an -newerXY test takes one too); those that hold true whatever
# the file, its actions among them; and its options after which -prune
# prunes nothing. fd's options that take a value.
FIND_NAME_TESTS = {
    "-name", "-iname", "-path", "-ipath", "-wholename", "-iwholename", "-regex", "-iregex",
}
FIND_ONE_ARGUMENT = {
    "-amin", "-anewer", "-atime", "-cmin", "-cnewer", "-context", "-ctime", "-fls", "-fprint",
    "-fprint0", "-fstype", "-gid", "-group", "-ilname", "-inum", "-links", "-lname", "-maxdepth",
    "-mindepth", "-mmin", "-mtime", "-newer", "-perm", "-printf", "-regextype", "-samefile",
    "-size", "-type", "-uid", "-used", "-user", "-xtype",
}
FIND_TRUE = {
    "-print", "-print0", "-printf", "-fprint", "-fprint0", "-fprintf", "-ls", "-fls", "-prune",
    "-true", "-depth", "-d", "-maxdepth", "-mindepth", "-xdev", "-mount", "-follow", "-noleaf",
    "-regextype", "-daystart", "-warn", "-nowarn", "-ignore_readdir_race",
    "-noignore_readdir_race",
}
FIND_NO_PRUNE = {"-depth", "-d", "-delete"}
FD_SHORT_VALUES = set("etEdSojc")
FD_LONG_VALUES = {
    "--extension", "--type", "--exclude", "--max-depth", "--min-depth", "--exact-depth", "--size",
    "--changed-within", "--changed-before", "--owner", "--threads", "--max-results", "--color",
    "--base-directory", "--path-separator", "--search-path", "--format", "--batch-size",
    "--ignore-file",
}
# Words that open and close a compound command, to follow a pipe into a loop
# (find . | while read f; do rm "$f"; done).
BLOCK_OPENERS = {"while", "until", "for", "select", "if", "case", "{"}
BLOCK_CLOSERS = {"done", "fi", "esac", "}"}
# ssh options whose value is the next word, and the -o options that run a
# command (ssh -o ProxyCommand='...').
SSH_VALUE_OPTIONS = set("-B -b -c -D -E -e -F -I -i -J -L -l -m -O -o -p -Q -R -S -W -w".split())
SSH_COMMAND_OPTION = re.compile(
    r"(?i)\s*(?:proxy|local|remote|knownhosts)command\s*[=\s]\s*(.*)", re.S
)
# Programs that run their arguments as shell commands.
SHELLS = {"sh", "bash", "zsh", "dash", "ksh", "fish"}
# Programs whose arguments this guard checks.
CHECKED_PROGRAMS = (
    {"git", "rm", "unlink", "shred", "eval", "ssh"} | FINDERS | SHELLS | MULTICALL | READERS
)
# git global options whose value is the next argument.
GIT_VALUE_OPTIONS = {
    "-C", "-c", "--git-dir", "--work-tree", "--namespace", "--super-prefix", "--config-env",
    "--attr-source",
}
# git global options that change which configuration git reads, and so its
# aliases. The guard passes them on when it looks an alias up.
GIT_CONFIG_OPTIONS = {"-C", "-c", "--config-env", "--git-dir", "--bare"}
# Commands git ships, which no alias can replace: git looks for an alias only
# when it has no command of that name. The guard asks git about the others.
GIT_COMMANDS = set("""
    add am apply bisect blame branch cat-file checkout cherry-pick clean clone commit config
    describe diff fetch format-patch grep help init log ls-files ls-remote ls-tree merge
    merge-base mv notes pull push rebase reflog remote reset restore rev-list rev-parse revert rm
    shortlog show show-ref stash status submodule switch tag worktree
""".split())
# Changing directory or git's configuration environment, after which an
# alias can mean something else than where the hook looked it up.
CONTEXT_CHANGE = re.compile(
    r"(?<![\w./-])(cd|pushd|popd)(?![\w./-])|\b(GIT_\w*|HOME|XDG_CONFIG_HOME)="
)

ASSIGNMENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*=")
DURATION = re.compile(r"[0-9.]+[smhd]?$")
SUBSTITUTION = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")
# An rm argument that can expand to options: an option built from a
# variable, or the positional parameters, through which a function or a git
# shell alias passes its arguments.
UNCHECKABLE_RM_ARGUMENT = re.compile(r"-.*[$`]|\$\{?[@*1-9]")
# Operators that end a command. Redirections (<, >) do not.
SEPARATOR_CHARS = set(";&|()\n")
OPERATOR_CHARS = SEPARATOR_CHARS | set("<>")
FALLBACK_TOKEN = re.compile(r"[;&|()<>\n]+|[^\s;&|()<>]+")
# Stand-ins for a quoted or escaped parenthesis while shlex splits a command.
HIDDEN_PARENTHESES = {"(": "\ue000", ")": "\ue001"}
SHOWN_PARENTHESES = str.maketrans({"\ue000": "(", "\ue001": ")"})
MAX_DEPTH = 8
# The most words a brace expansion may make (roko-std's EXPANSION_LIMIT).
BRACE_LIMIT = 4096

# The Bash call being checked (check_bash sets it): the directory git
# aliases are looked up in, and the whole command. dir is the directory the
# command being checked runs in, after the cd commands before it, and dirs
# every directory the call may run a command in. bulk is set, to why a
# delete may not run, while a command that runs on a list the guard cannot
# see is checked: what find or fd lists, or what xargs reads; list is then
# where that list comes from, as ([roots], reads) (see check_tree_reads).
BASH_CALL = {"cwd": None, "dir": None, "dirs": [], "command": "", "bulk": None, "list": None}
FIND_BULK = "rm on what find or fd lists is forbidden: it deletes files across a whole tree"
XARGS_BULK = (
    "a delete run by xargs is forbidden: it deletes every file in a list the guard cannot see"
)
PARALLEL_BULK = "a delete run by parallel is forbidden: it deletes every file in its list"
# Wrappers that run their command on each item of a list.
BULK_RUNNERS = {"xargs": XARGS_BULK, "parallel": PARALLEL_BULK}


def tokens(text):
    """The words and operators of `text`. A quoted or escaped parenthesis
    (find . \\( -name x \\)) belongs to its word, where shlex would make an
    operator of it, so it stays hidden: shown() turns a word back."""
    lexer = shlex.shlex(hide_quoted_parentheses(text), posix=True, punctuation_chars="();<>|&\n")
    lexer.whitespace = " \t\r"
    lexer.whitespace_split = True
    # A shell starts a comment only at the start of a word; shlex would
    # also start one inside a word and hide the rest of the line.
    lexer.commenters = ""
    try:
        return list(lexer)
    except ValueError:
        # Unbalanced quotes, as in a heredoc: split on whitespace and
        # operators instead, and drop the quotes.
        return [token.strip("\"'") for token in FALLBACK_TOKEN.findall(text)]


def hide_quoted_parentheses(text):
    """`text` with each parenthesis in quotes or after a backslash replaced
    by a stand-in (see tokens)."""
    hidden, quote, escaped = [], None, False
    for character in text:
        if escaped:
            escaped = False
            character = HIDDEN_PARENTHESES.get(character, character)
        elif character == "\\" and quote != "'":
            escaped = True
        elif quote:
            quote = None if character == quote else quote
            character = HIDDEN_PARENTHESES.get(character, character)
        elif character in "'\"":
            quote = character
        hidden.append(character)
    return "".join(hidden)


def shown(word):
    """A word of tokens with its hidden parentheses shown."""
    return word.translate(SHOWN_PARENTHESES)


def simple_commands(text):
    """The commands in `text`, split at its operators, as (words, fed)
    pairs: fed, the words of the find or fd command, when a pipe brings the
    command what it lists, directly or into a loop (find . | while read f;
    do rm "$f"; done)."""
    commands, words = [], []
    finds = fed = None
    blocks = 0  # compound commands opened since the pipe
    for token in tokens(text.replace("\\\n", " ")):
        characters = set(token)
        if not (characters and characters <= OPERATOR_CHARS and characters & SEPARATOR_CHARS):
            if fed and not command_start(words):
                if token in BLOCK_OPENERS:
                    blocks += 1
                elif token in BLOCK_CLOSERS and blocks:
                    blocks -= 1
            words.append(shown(token))
            continue
        if words:
            commands.append((words, fed))
            finds = finds or (words if runs_finder(words) else None)
        words = []
        if token in ("|", "|&"):
            fed = fed or finds
        elif token == "(":
            if fed:
                blocks += 1
        elif token == ")":
            if fed and blocks:
                blocks -= 1
        elif not blocks:
            finds = fed = None
    if words:
        commands.append((words, fed))
    return commands


def check_command(text, depth=0):
    if depth > MAX_DEPTH:
        block("the command nests shells too deeply to check")
    # A nested command (sh -c '...', $(...)) runs in the directory it starts
    # in, and its cd does not move the commands after it.
    start = BASH_CALL["dir"]
    # A substitution can hand a command what find lists (rm $(find . -name x)).
    listed = None
    for match in SUBSTITUTION.finditer(text):
        inner = match.group(1) if match.group(1) is not None else match.group(2)
        check_command(inner, depth + 1)
        finders = (words for words, _ in simple_commands(inner) if runs_finder(words))
        listed = listed or next(finders, None)
    for words, fed in simple_commands(text):
        if fed or listed:
            check_found(words, depth, source=fed or listed)
        else:
            check_words(words, depth)
        change_directory(words)
    if depth:
        BASH_CALL["dir"] = start


def cd_target(words):
    """The directory `words` moves to when it is a cd or pushd to a literal
    one (home when it names none), else None: cd - and a variable are
    unknown."""
    words = command_start(words)
    if not words or program_name(words[0]) not in ("cd", "pushd") or "-" in words:
        return None
    targets = [word for word in words[1:] if not word.startswith("-")]
    target = targets[0] if targets else "~"
    return None if re.search(r"[$`]", target) else os.path.expanduser(target)


def change_directory(words):
    """Follow `words` when it is a cd or pushd, so the commands after it
    are judged where they run."""
    target = cd_target(words)
    if target is not None and BASH_CALL["dir"] is not None:
        BASH_CALL["dir"] = os.path.normpath(os.path.join(BASH_CALL["dir"], target))


def call_directories(command, cwd, depth=0):
    """The directories the commands of a Bash call may run in: cwd, and each
    literal cd or pushd target in it, nested command lines included."""
    directories, current = [cwd], cwd
    for words, _ in simple_commands(command):
        target = cd_target(words)
        if target is not None:
            current = os.path.normpath(os.path.join(current, target))
            directories.append(current)
        for word in words:
            if depth < MAX_DEPTH and re.search(r"\s", word):
                directories += call_directories(word, current, depth + 1)[1:]
    return directories


def program_name(word):
    return os.path.basename(word)


def command_start(words):
    """`words` without the keywords and assignments that can precede a program."""
    while words and (words[0] in SHELL_KEYWORDS or ASSIGNMENT.match(words[0])):
        words = words[1:]
    return words


def runs_finder(words):
    """Whether a command runs find or fd, directly or through a wrapper."""
    words = command_start(words)
    if not words:
        return False
    program = program_name(words[0])
    if program in WRAPPERS or program in MULTICALL:
        return any(program_name(word) in FINDERS for word in words[1:])
    return program in FINDERS


def check_words(words, depth, maybe_argument=False):
    """Check one command. `maybe_argument`: its first word may be an
    argument of a wrapper rather than the program the wrapper runs."""
    words = command_start(words)
    if not words:
        return
    program = program_name(words[0])
    if program in READERS and BASH_CALL["list"]:
        check_listed_read()
    if program in MULTICALL:
        check_words(words[1:], depth, maybe_argument)
    elif program in WRAPPERS:
        check_wrapped(program, words, depth, maybe_argument)
    elif program == "git":
        check_git(words[1:], depth, maybe_argument)
    elif program == "rm":
        check_rm(words[1:])
    elif program in ("unlink", "shred") and BASH_CALL["bulk"]:
        block(BASH_CALL["bulk"])
    elif program == "find":
        check_find(words[1:], depth)
    elif program in FINDERS:
        check_fd(words[1:], depth)
    elif program == "ssh":
        check_ssh(words[1:], depth)
    elif program in SEARCHERS:
        check_search(program, words[1:])
    elif program in SHELLS:
        for argument in words[1:]:
            if not argument.startswith("-"):
                check_command(argument, depth + 1)
    elif program == "eval":
        check_command(" ".join(words[1:]), depth + 1)


def check_wrapped(program, words, depth, maybe_argument=False):
    """Check the commands a wrapper (`words[0]`, named `program`) runs.

    Its own options come first, and an option's value can name a program too
    (xargs -a git rm -rf x), so every later word that names a program this
    guard checks, or another wrapper, starts a command to check. A user or
    group is skipped (sudo -u git whoami), and so are the words git itself
    reads: its global options' values and its subcommand (sudo git -C dir
    rm -r x, xargs git rm --cached), which git checks. After a word that is
    not an option, an assignment, a number (timeout's duration), a user or
    another wrapper, the word may be an argument instead (timeout 5 grep git
    src). A command handed over as one string (watch 'rm -rf x', flock l -c
    '...') is checked as a command line. xargs and parallel run their command
    on each item of a list, so a delete in it is denied."""
    with bulk(BULK_RUNNERS.get(program)):
        after_argument = maybe_argument
        git_words = set()
        for index in range(1, len(words)):
            if index in git_words:
                continue
            word, previous = words[index], words[index - 1]
            is_user = previous in USER_OPTIONS
            name = program_name(word)
            # A git that is not an option's value (xargs -a git rm -rf x)
            # reads the words after it up to its subcommand.
            if name == "git" and previous not in VALUE_OPTIONS:
                git_words |= git_argument_indices(words, index)
            if (name in CHECKED_PROGRAMS or name in WRAPPERS) and not is_user:
                check_words(words[index:], depth, after_argument)
            option, _, value = word.partition("=")
            if program in STRING_WRAPPERS or previous in COMMAND_OPTIONS:
                check_command(word, depth + 1)
            elif option in COMMAND_OPTIONS and value:
                check_command(value, depth + 1)
            if not (
                word.startswith("-") or ASSIGNMENT.match(word) or DURATION.match(word) or is_user
                or program_name(word) in WRAPPERS
            ):
                after_argument = True


def git_argument_indices(words, index):
    """The indices of the words that the git at words[index] reads before its
    subcommand's arguments: its global options' values (-C dir) and the
    subcommand itself."""
    indices = set()
    index += 1
    while index < len(words) and words[index].startswith("-"):
        if words[index] in GIT_VALUE_OPTIONS:
            indices.add(index + 1)
            index += 2
        else:
            index += 1
    indices.add(index)
    return indices


@contextlib.contextmanager
def bulk(reason, source=None):
    """While the block checks a command that runs on a list the guard cannot
    see, deny any delete in it, for `reason`, and judge a read in it by where
    the list comes from: `source`, as (roots, reads) (see check_tree_reads),
    or what ls lists in the call's directory. None changes nothing."""
    previous = BASH_CALL["bulk"], BASH_CALL["list"]
    if reason:
        BASH_CALL["bulk"] = previous[0] or reason
        BASH_CALL["list"] = previous[1] or source or (["."], lists_visible)
    try:
        yield
    finally:
        BASH_CALL["bulk"], BASH_CALL["list"] = previous


def check_found(command, depth, reason=FIND_BULK, maybe_argument=False, source=None):
    """Check a command that runs on a list the guard cannot see: what find
    or fd lists (find -exec, fd -x, find . | xargs), `source` being the
    finder's words when known. Any delete in it is denied, for `reason`."""
    with bulk(reason, finder_list(source) if source else None):
        check_words(command, depth, maybe_argument)


def check_listed_read():
    """Deny a read (cat, grep) run on a list the guard cannot see when the
    list may name a key file or a roko config file that holds a secret, in
    the tree it comes from (check_tree_reads)."""
    roots, reads = BASH_CALL["list"]
    check_tree_reads(roots, BASH_CALL["dir"] or os.getcwd(), reads)


def lists_visible(rel, start, path):
    """Whether a list the guard cannot see (ls | xargs cat, xargs cat <
    list) may name the file at `rel`: one no hidden directory leads to, as
    ls -a lists them."""
    return not any(part.startswith(".") for part in rel.split("/")[:-1])


def finder_list(words):
    """Where the list a find or fd command prints comes from: its starting
    points, and what it lists from them (see check_tree_reads)."""
    words = command_start(words)
    for index, word in enumerate(words):
        name = program_name(word)
        if name == "find":
            return find_list(words[index + 1:])
        if name in FINDERS:
            return fd_list(words[index + 1:])
    return None


def find_list(arguments):
    """find's starting points, and what it lists from them (see
    check_tree_reads): a file its expression may hold true, under no
    directory the expression surely prunes, within -maxdepth and -mindepth.
    An expression the guard cannot parse lists every file."""
    index = 0
    while index < len(arguments) and (
        arguments[index] in ("-H", "-L", "-P", "-D") or arguments[index].startswith("-O")
    ):
        index += 2 if arguments[index] == "-D" else 1
    roots = []
    for argument in arguments[index:]:
        if argument.startswith("-") or argument in ("(", "!", ","):
            break
        roots.append(argument)
    words = arguments[index + len(roots):]
    try:
        expression = find_expression(words)
    except ValueError:
        expression = None
    limits = {
        word: int(words[position + 1])
        for position, word in enumerate(words[:-1])
        if word in ("-maxdepth", "-mindepth") and words[position + 1].isdigit()
    }
    prunes = not FIND_NO_PRUNE & set(words)

    def lists(rel, start, path):
        parts = rel.split("/")
        if not limits.get("-mindepth", 0) <= len(parts) <= limits.get("-maxdepth", len(parts)):
            return False
        if expression is None:
            return True
        for count in range(1, len(parts)):
            directory = printed_path(start, "/".join(parts[:count]))
            if prunes and find_evaluate(expression, directory, "d")[1]:
                return False
        return find_evaluate(expression, printed_path(start, rel), file_kind(path))[0] is not False

    return roots or ["."], lists


def find_expression(words):
    """find's expression (`words`, after its starting points) as a tree of
    tuples: ("and" | "or" | ",", left, right), ("not", operand), or
    ("primary", name, argument), the argument of -exec being its ; or +.
    Raises ValueError when the words do not parse."""
    position = 0

    def primary():
        nonlocal position
        if position >= len(words):
            raise ValueError("find's expression ends early")
        word = words[position]
        position += 1
        if word in ("!", "-not"):
            return ("not", primary())
        if word == "(":
            node = alternatives()
            if position >= len(words) or words[position] != ")":
                raise ValueError("find's expression leaves a parenthesis open")
            position += 1
            return node
        argument = None
        if word in FIND_EXEC:
            while position < len(words) and words[position] not in (";", "+"):
                position += 1
            argument = words[position] if position < len(words) else ";"
            position += 1
        elif word in FIND_NAME_TESTS or word in FIND_ONE_ARGUMENT or word.startswith("-newer"):
            argument = words[position] if position < len(words) else ""
            position += 1
        elif word == "-fprintf":
            position += 2
        return ("primary", word, argument)

    def conjunction():
        nonlocal position
        node = primary()
        while position < len(words) and words[position] not in ("-o", "-or", ",", ")"):
            if words[position] in ("-a", "-and"):
                position += 1
            node = ("and", node, primary())
        return node

    def disjunction():
        nonlocal position
        node = conjunction()
        while position < len(words) and words[position] in ("-o", "-or"):
            position += 1
            node = ("or", node, conjunction())
        return node

    def alternatives():
        nonlocal position
        node = disjunction()
        while position < len(words) and words[position] == ",":
            position += 1
            node = (",", node, disjunction())
        return node

    if not words:
        return ("primary", "-true", None)
    node = alternatives()
    if position < len(words):
        raise ValueError("find's expression closes a parenthesis it never opened")
    return node


def find_evaluate(node, path, kind):
    """What find's expression `node` gives for the entry it prints as
    `path`, of file type `kind` (find's letter, None when unknown): True,
    False, or None when the guard cannot tell; and whether it surely prunes
    the entry. An operand that find may skip (after a true -o) prunes
    nothing for sure."""
    operator = node[0]
    if operator == "primary":
        return find_primary(node[1], node[2], path, kind), node[1] == "-prune"
    if operator == "not":
        value, prunes = find_evaluate(node[1], path, kind)
        return (None if value is None else not value), prunes
    left, prunes = find_evaluate(node[1], path, kind)
    # Whether the right operand runs: after a comma always, after -a when
    # the left one is true, after -o when it is false.
    if operator == ",":
        runs = True
    elif operator == "and":
        runs = left
    else:
        runs = None if left is None else not left
    if runs is False:
        return left, prunes
    right, right_prunes = find_evaluate(node[2], path, kind)
    prunes = prunes or (runs is True and right_prunes)
    if runs is True:
        return right, prunes
    # The left operand is unknown: only the right one can decide.
    decided = operator == "or"
    return (decided if right is decided else None), prunes


def find_primary(name, argument, path, kind):
    """What a find primary gives for the entry printed as `path`: its name,
    path and type tests as find decides them; True for actions and options;
    None for -exec ... ; and the tests the guard does not model. find's
    regexes are emacs regexes matched against the whole path, so a miss
    there proves nothing."""
    if name in ("-name", "-iname", "-path", "-ipath", "-wholename", "-iwholename"):
        text = path.rsplit("/", 1)[-1] if name in ("-name", "-iname") else path
        if name.startswith("-i"):
            text, argument = text.lower(), argument.lower()
        return fnmatch.fnmatchcase(text, argument)
    if name in ("-regex", "-iregex"):
        try:
            return True if re.fullmatch(argument, path, re.I if name == "-iregex" else 0) else None
        except re.error:
            return None
    if name == "-type":
        return None if kind is None else kind in argument.split(",")
    if name in FIND_EXEC:
        return True if argument == "+" else None
    if name in FIND_TRUE:
        return True
    return False if name == "-false" else None


def printed_path(start, rel):
    """How find prints the entry at `rel` under the starting point `start`."""
    return start.rstrip("/") + "/" + rel


def file_kind(path):
    """find's type letter (f, d or l) for the file at `path`, or None."""
    try:
        mode = os.lstat(path).st_mode
    except OSError:
        return None
    if stat.S_ISDIR(mode):
        return "d"
    if stat.S_ISLNK(mode):
        return "l"
    return "f" if stat.S_ISREG(mode) else None


def fd_list(arguments):
    """fd's search paths, and what it lists from them (see
    check_tree_reads): no hidden file without -H or -u, nothing an -E glob
    leaves out or deeper than -d, and only the names its pattern (a regex, a
    glob with -g, a string with -F, the whole path with -p) and its
    extensions (-e) match, case ignored."""
    options, operands = search_arguments(arguments, FD_SHORT_VALUES, FD_LONG_VALUES)
    names = {name for name, _ in options}
    roots = operands[1:] + [value for name, value in options if name == "--search-path"]
    pattern = operands[0] if operands else ""
    extensions = [
        value.lower().lstrip(".") for name, value in options if name in ("-e", "--extension")
    ]
    hidden = bool(names & {"-H", "--hidden", "-u", "--unrestricted"})

    def lists(rel, start, path):
        parts = rel.split("/")
        name = parts[-1].lower()
        if not hidden and any(part.startswith(".") for part in parts):
            return False
        if extensions and not any(name.endswith("." + extension) for extension in extensions):
            return False
        for option, value in options:
            if option in ("-E", "--exclude") and glob_excludes(value, rel, False):
                return False
            if option in ("-d", "--max-depth") and value.isdigit() and len(parts) > int(value):
                return False
        if not pattern:
            return True
        target = path.lower() if names & {"-p", "--full-path"} else name
        if names & {"-g", "--glob"}:
            return fnmatch.fnmatchcase(target, pattern.lower())
        if names & {"-F", "--fixed-strings"}:
            return pattern.lower() in target
        try:
            return re.search(pattern, target, re.I) is not None
        except re.error:
            return True

    return roots or ["."], lists


def check_find(arguments, depth):
    if "-delete" in arguments:
        block("find -delete forbidden: it deletes whole directory trees")
    for index, argument in enumerate(arguments):
        if argument in FIND_EXEC:
            command = []
            for word in arguments[index + 1:]:
                if word in (";", "+"):
                    break
                command.append(word)
            check_found(command, depth, source=["find"] + arguments)


def check_fd(arguments, depth):
    # fd -x runs every word after it, up to a ;, on each result.
    for index, argument in enumerate(arguments):
        option, _, value = argument.partition("=")
        if option in FD_EXEC:
            command = ([value] if value else []) + arguments[index + 1:]
            check_found(command, depth, source=["fd"] + arguments[:index])
            return


def check_ssh(arguments, depth):
    """ssh [options] destination [command ...]: ssh joins the command's
    words for the remote shell, and -o ProxyCommand and its kind run one."""
    index = 0
    while index < len(arguments) and arguments[index].startswith("-"):
        option = arguments[index]
        if option in SSH_VALUE_OPTIONS:
            value = arguments[index + 1] if index + 1 < len(arguments) else ""
            index += 2
        else:
            value = option[2:]  # a value given in the same word (-oProxyCommand=...)
            index += 1
        command = SSH_COMMAND_OPTION.match(value)
        if command:
            check_command(command.group(1), depth + 1)
    if index + 1 < len(arguments):
        check_command(" ".join(arguments[index + 1:]), depth + 1)


def check_search(program, arguments):
    """Deny a recursive search (grep -r, rg, ag, ack) that would read a key
    file or a roko config file holding a secret (search_reads)."""
    kind = program if program in SEARCH_OPTIONS else "grep"
    short_values, long_values, lists_names, pattern_options = SEARCH_OPTIONS[kind]
    options, operands = search_arguments(arguments, short_values, long_values)
    names = {name for name, _ in options}
    if names & lists_names:
        return  # lists file names, reads no file
    if kind == "grep" and program != "rgrep" and not (
        names & {"-r", "-R", "--recursive", "--dereference-recursive"}
        or any(name in ("-d", "--directories") and value == "recurse" for name, value in options)
    ):
        return  # a file it names is checked with the other words
    if not names & pattern_options:
        operands = operands[1:]  # the first is the pattern
    check_tree_reads(
        operands or ["."], BASH_CALL["dir"] or os.getcwd(),
        lambda rel, start, path: search_reads(options, kind, rel),
    )


def check_tree_reads(roots, directory, reads):
    """Deny a read of the trees at `roots`, resolved against `directory`,
    that reaches a file agents must not read (sensitive_files) in one:
    reads(rel, start, path) tells whether it reads the file at `path`, `rel`
    being its path under the root and `start` the root as the program sees
    it. A brace or a glob in a root is expanded as the shell would."""
    for root in roots:
        for variant in expand_braces(root):
            if re.search(r"[$`{]", variant):
                continue
            word = os.path.expanduser(variant)
            for path in expand(variant, directory):
                if not os.path.isdir(path):
                    if is_secret_config_path(path, directory):
                        block(CONFIG_SECRET_REASON)
                    continue
                start = word
                if re.search(r"[*?\[]", variant) and not os.path.isabs(word):
                    # The shell passes a glob's match as the glob gives it.
                    start = os.path.relpath(path, directory)
                    start = "./" + start if word.startswith("./") else start
                top = os.path.realpath(path)
                for found, reason in sensitive_files(top, directory):
                    if reads(os.path.relpath(found, top), start, found):
                        block(reason)


def check_git_grep(arguments, options):
    """git grep reads the tracked files under the directory git runs in (git
    -C), or under the paths and pathspecs it names, down to --max-depth, and
    the untracked ones too with --untracked or --no-index: a key file is
    never tracked. A revision it names reads that commit's tree of the same
    directory."""
    grep_options, operands = search_arguments(arguments, set("efmABC"), GIT_GREP_LONG_VALUES)
    names = {name for name, _ in grep_options}
    if not names & {"-e", "-f"}:
        operands = operands[1:]  # the first is the pattern
    directory = BASH_CALL["dir"] or os.getcwd()
    for position, option in enumerate(options[:-1]):
        if option == "-C":
            directory = os.path.join(directory, os.path.expanduser(options[position + 1]))
    globs = [operand for operand in operands if re.search(r"[*?\[]", operand)]
    paths = [
        operand for operand in operands
        if operand not in globs and os.path.exists(os.path.join(directory, operand))
    ]
    untracked = bool(names & {"--untracked", "--no-index"})
    depths = [
        int(value) for name, value in grep_options if name == "--max-depth" and value.isdigit()
    ]

    def reads(narrowed):
        def git_reads(rel, start, path):
            if not untracked and is_key_path(path, directory, False):
                return False
            if depths and rel.count("/") > depths[-1]:
                return False
            return not narrowed or any(glob_includes(pattern, rel) for pattern in globs)
        return git_reads

    if paths or not globs:
        check_tree_reads(paths or ["."], directory, reads(False))
    if globs:
        check_tree_reads(["."], directory, reads(True))


def search_arguments(arguments, short_values, long_values):
    """A search command's options, as (name, value) pairs, and its operands.
    `short_values` and `long_values` are the options that take a value."""
    options, operands = [], []
    index = 0
    while index < len(arguments):
        word = arguments[index]
        index += 1
        if word == "--":
            operands += arguments[index:]
            break
        if word.startswith("--"):
            name, equals, value = word.partition("=")
            if not equals and name in long_values and index < len(arguments):
                value, index = arguments[index], index + 1
            options.append((name, value))
        elif word.startswith("-") and len(word) > 1:
            # A cluster of short options; one that takes a value ends it
            # (-rnm1, -re PATTERN).
            for position, letter in enumerate(word[1:], 2):
                value = ""
                if letter in short_values:
                    value = word[position:]
                    if not value and index < len(arguments):
                        value, index = arguments[index], index + 1
                options.append(("-" + letter, value))
                if letter in short_values:
                    break
        else:
            operands.append(word)
    return options, operands


def search_reads(options, kind, rel):
    """Whether a recursive search reads the file at `rel` in the tree it
    searches. grep and ack read hidden files and directories; rg and ag do
    with --hidden or -uu (ag -u), and rg with a glob that names one. Filters
    can leave the file out: grep's --include, --exclude and --exclude-dir;
    rg's -g (! excludes), -t, -T and --max-depth; ag's -G, --ignore and
    --depth; ack's -t, --ignore-dir and --ignore-file."""
    parts = rel.split("/")
    name = parts[-1]
    includes, types, skipped_types = [], set(), set()
    hidden, unrestricted, typed = kind in ("grep", "ack"), 0, True
    for option, value in options:
        if kind == "grep" and option == "--include" or (
            kind == "rg" and option in RG_GLOBS and not value.startswith("!")
        ):
            includes.append(value)
        elif kind == "rg" and option in RG_GLOBS or (
            kind == "ag" and option in ("--ignore", "--ignore-dir")
        ):
            if glob_excludes(value[1:] if kind == "rg" else value, rel, option == "--iglob"):
                return False
        elif kind == "grep" and option == "--exclude" and fnmatch.fnmatchcase(name, value):
            return False
        elif kind == "grep" and option == "--exclude-dir" and any(
            fnmatch.fnmatchcase(part, value.rstrip("/")) for part in parts[:-1]
        ):
            return False
        elif kind == "ack" and option == "--ignore-dir" and any(
            ack_filter(value, part) for part in parts[:-1]
        ):
            return False
        elif kind == "ack" and option == "--ignore-file" and ack_filter(value, name):
            return False
        elif kind == "ag" and option in ("-G", "--file-search-regex"):
            try:
                if re.search(value, rel) is None:
                    return False
            except re.error:
                pass
        elif kind in ("rg", "ack") and option in ("-t", "--type"):
            if value.startswith("no"):
                skipped_types.add(value[2:])
            else:
                types.add(value)
        elif kind == "rg" and option in ("-T", "--type-not"):
            skipped_types.add(value)
        elif kind == "rg" and option == "--type-add":
            typed = False  # a type it adds may cover the file
        elif kind in ("rg", "ag") and option == "--hidden" or kind == "rg" and option == "-.":
            hidden = True
        elif kind == "rg" and option == "--no-hidden":
            hidden = False
        elif kind in ("rg", "ag") and option in ("-u", "--unrestricted"):
            unrestricted += 1
        elif (
            kind == "rg" and option in ("-d", "--max-depth", "--maxdepth")
            or kind == "ag" and option == "--depth"
        ) and value.isdigit():
            # rg counts the files in the root as one level deep, ag as none.
            if len(parts) - (kind == "ag") > int(value):
                return False
    hidden = hidden or unrestricted >= (2 if kind == "rg" else 1)
    for count, part in enumerate(parts, 1):
        if part.startswith(".") and not hidden and not (
            kind == "rg"
            and any(glob_includes(pattern, "/".join(parts[:count])) for pattern in includes)
        ):
            return False
    if includes and not any(glob_includes(pattern, rel) for pattern in includes):
        return False
    types_of_file = file_types(name)
    if typed and types and not types & (types_of_file | {"all"}):
        return False
    return not skipped_types & types_of_file


def file_types(name):
    """The rg file types (rg --type-list) of a key file or roko config file
    named `name`: toml, json, and sh for a .env file."""
    if name.endswith(".toml"):
        return {"toml"}
    if name.endswith(".json"):
        return {"json"}
    return {"sh"} if name.endswith(".env") else set()


def ack_filter(value, name):
    """Whether ack's filter (is:NAME, ext:EXT,..., match:/REGEX/, or a bare
    name) matches the file or directory `name`."""
    kind, colon, argument = value.partition(":")
    if not colon:
        return name == value
    if kind == "is":
        return name == argument
    if kind == "ext":
        return "." in name and name.rsplit(".", 1)[1] in argument.split(",")
    if kind == "match":
        try:
            return re.search(argument.strip("/"), name) is not None
        except re.error:
            return False
    return False


def glob_includes(pattern, rel):
    """Whether a glob that lets files in (grep --include, rg -g, a git
    pathspec) may match the file at `rel`, by name or by path. Braces are
    expanded and case ignored, so that a doubt counts as a match."""
    name = rel.rsplit("/", 1)[-1].lower()
    for variant in expand_braces(pattern.lower()):
        variant = variant.lstrip("/")
        while variant.startswith("**/"):
            variant = variant[3:]
        if fnmatch.fnmatchcase(name, variant) or fnmatch.fnmatchcase(rel.lower(), variant):
            return True
    return False


def glob_excludes(pattern, rel, fold):
    """Whether a glob that leaves files out (rg -g '!...', ag --ignore, fd
    -E) surely matches the file at `rel` or a directory it is in, by name or
    by path from the root; `fold` ignores case (rg --iglob)."""
    pattern = pattern.lstrip("/").rstrip("/")
    while pattern.startswith("**/"):
        pattern = pattern[3:]
    if fold:
        pattern, rel = pattern.lower(), rel.lower()
    parts = rel.split("/")
    return any(fnmatch.fnmatchcase(part, pattern) for part in parts) or any(
        fnmatch.fnmatchcase("/".join(parts[:count]), pattern) for count in range(1, len(parts) + 1)
    )


def short_flags(arguments):
    """The letters of the short-option clusters in `arguments` (-fdx gives f, d, x)."""
    letters = set()
    for argument in arguments:
        if argument == "--":
            break
        if len(argument) > 1 and argument[0] == "-" and argument[1] != "-":
            letters.update(argument[1:])
    return letters


def check_rm(arguments):
    if BASH_CALL["bulk"]:
        block(BASH_CALL["bulk"])
    # GNU rm reads options anywhere before --, and accepts a long option
    # shortened to any unambiguous prefix (--rec).
    options = arguments[:arguments.index("--")] if "--" in arguments else arguments
    if {"r", "R"} & short_flags(options) or any(
        len(option) > 2 and "--recursive".startswith(option) for option in options
    ):
        block("recursive rm forbidden: it deletes whole directory trees")
    if any(UNCHECKABLE_RM_ARGUMENT.match(option) for option in options):
        block("rm options taken from a variable cannot be checked")


def check_git(arguments, depth, maybe_argument=False):
    index, options = 0, []
    while index < len(arguments) and arguments[index].startswith("-"):
        width = 2 if arguments[index] in GIT_VALUE_OPTIONS else 1
        if arguments[index].split("=", 1)[0] in GIT_CONFIG_OPTIONS:
            options += arguments[index:index + width]
        index += width
    if index >= len(arguments):
        return
    subcommand, rest = arguments[index], arguments[index + 1:]
    if "$" in subcommand or "`" in subcommand:
        block("a git subcommand taken from a variable cannot be checked")
    try:
        alias = git_alias(subcommand, options)
    except Unresolved as error:
        # Where git may be an argument (timeout 5 grep git src), its next
        # word need not be a git command at all.
        if maybe_argument:
            return
        block(str(error))
    if alias is not None:
        check_git_alias(subcommand, alias, options, rest, depth)
        return
    if subcommand == "grep":
        check_git_grep(rest, options)
    flags = short_flags(rest)
    if subcommand == "checkout":
        block("git checkout forbidden: agents must not switch branches or discard changes")
    if subcommand == "switch":
        block("git switch forbidden: agents must not switch branches")
    if subcommand == "restore":
        block("git restore forbidden: it discards uncommitted changes")
    if subcommand == "push":
        block("agents must not push - roko handles merges")
    if subcommand == "branch":
        deletes = "d" in flags or "--delete" in rest
        forced = "f" in flags or "--force" in rest
        if flags & {"m", "M", "D"} or "--move" in rest or (deletes and forced):
            block("branch rename or force delete forbidden: roko manages branches")
    if subcommand == "reset" and "--hard" in rest:
        block("git reset --hard forbidden: it discards uncommitted changes")
    if subcommand == "stash" and (not rest or rest[0] not in ("list", "show")):
        block("git stash forbidden (stash list and stash show are allowed): it can lose uncommitted work")
    if subcommand == "clean" and "n" not in flags and "--dry-run" not in rest:
        block("git clean forbidden (dry runs with -n are allowed): it deletes untracked files")


def check_git_alias(name, alias, options, rest, depth):
    """Check `git <name> <rest>`, which runs `alias`."""
    if depth >= MAX_DEPTH:
        block("git alias " + name + " nests too deeply to check")
    if alias.startswith("!"):
        # git runs a shell alias with sh, the arguments appended.
        check_command(" ".join([alias[1:]] + [shlex.quote(word) for word in rest]), depth + 1)
    else:
        try:
            words = shlex.split(alias)
        except ValueError:
            block("git alias " + name + " cannot be parsed")
        check_git(options + words + rest, depth + 1)
    if CONTEXT_CHANGE.search(BASH_CALL["command"]):
        block("git alias " + name + " cannot be checked where the command changes directory or git's environment")


class Unresolved(Exception):
    """git cannot tell what a subcommand runs."""


def git_alias(name, options):
    """The alias `git <name>` runs, with the command's global `options`, or
    None when name is one of git's commands. Raises Unresolved when name is
    neither, or git cannot tell."""
    if name in GIT_COMMANDS or name in git_commands():
        return None
    result = run_git(options + ["config", "--get", "alias." + name])
    if result.returncode == 1 and not result.stderr:
        raise Unresolved("git " + name + " is neither a git command nor an alias here, so it cannot be checked")
    if result.returncode != 0:
        raise Unresolved("git alias " + name + " cannot be resolved: " + git_failure(result))
    return result.stdout.rstrip("\n")


@functools.lru_cache(maxsize=None)
def git_commands():
    """The commands git runs rather than an alias: its own and git-* programs on PATH."""
    result = run_git(["--list-cmds=main,others"])
    if result.returncode != 0:
        raise Unresolved("git's commands cannot be listed: " + git_failure(result))
    return frozenset(result.stdout.split())


def run_git(arguments):
    """Run git where the Bash call runs."""
    try:
        return subprocess.run(
            ["git"] + arguments, cwd=BASH_CALL["cwd"], stdin=subprocess.DEVNULL,
            capture_output=True, text=True, timeout=10,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise Unresolved("git cannot run to resolve an alias: %s" % (error,))


def git_failure(result):
    return result.stderr.strip() or "git exited %d" % result.returncode


# Files in a .roko directory, ~/.roko or a checkout's, that hold provider
# keys or roko credentials (roko_core::child_env::KEY_FILE_NAMES), the
# global config.toml included.
KEY_FILE_NAMES = (".env", "secrets.toml", "credentials.json", "config.toml")
KEY_FILE_REASON = (
    "provider key files are off limits to agents (.env, secrets.toml, credentials.json"
    " and config.toml in any .roko directory, ~/.roko included)"
)
KEY_TREE_REASON = (
    "this command reads the files of a tree that holds a provider key file (.env,"
    " secrets.toml, credentials.json or config.toml in a .roko directory); leave .roko out"
    " (grep --exclude-dir=.roko, rg -g '!.roko') or name a narrower tree"
)
# The most subdirectories a level in which the guard looks for a .roko
# directory below a tree's root (sensitive_files).
KEY_SEARCH_DIRS = 4096
# A key file's name, ending there (.env, not .envrc).
KEY_NAME = r"(?:\.env|secrets\.toml|credentials\.json|config\.toml)(?![\w-])"
# In a command's text: a path to a key file, or a glob directly in a .roko
# directory (.roko/*).
KEY_PATH_TEXT = re.compile(r"(?<![\w.-])\.roko/(?:" + KEY_NAME + r"|[^/\s;&|<>()]*[*?\[{])")
# A word naming a .roko directory itself (cd ~/.roko, D=.roko), and one that
# ends in a key file's name (.env, $D/secrets.toml) or is a bare glob (*).
ROKO_DIR_WORD = re.compile(r"(?<![\w.-])\.roko$")
KEY_NAME_WORD = re.compile(r"(?<![\w.-])" + KEY_NAME + r"$|^[^/]*[*?\[{][^/]*$")
CONFIG_SECRET_REASON = (
    "this roko config file holds a secret such as serve.auth.api_key, so agents may not read"
    " it; the operator can move secrets to ROKO__* variables in .roko/.env"
    " (ROKO__SERVE__AUTH__API_KEY)"
)
# Config keys that hold a secret (roko_core's SECRET_KEY_FRAGMENTS); one that
# ends in _env names an environment variable instead.
SECRET_CONFIG_KEY = re.compile(
    r"api_key|secret|token|password|credential|authorization|private_key|wallet_key|passphrase",
    re.I,
)
# key = "string", on its own line or in an inline table; a table header; an
# inline extra_headers table; and an agent.env pair ["NAME", "value"].
CONFIG_STRING = re.compile(r"""["']?([\w.-]+)["']?\s*=\s*(?:"([^"\n]*)"|'([^'\n]*)')""")
CONFIG_TABLE = re.compile(r"\s*\[\[?\s*([^\]]*?)\s*\]\]?")
INLINE_HEADERS = re.compile(r"extra_headers\s*=\s*\{([^}]*)\}")
ENV_PAIR = re.compile(r"""\[\s*"([^"]*)"\s*,\s*"([^"]*)"\s*\]""")
# Name segments of an environment variable that holds a credential
# (roko_core::child_env::SECRET_SEGMENTS).
SECRET_ENV_SEGMENTS = {
    "KEY", "KEYS", "APIKEY", "TOKEN", "AUTHTOKEN", "SECRET", "SECRETS", "PASSWORD", "PASSWD",
    "PASSPHRASE", "CREDENTIAL", "CREDENTIALS",
}


def command_words(text, depth=0):
    """The words of `text` with quotes removed, each brace expansion's words
    as well (roko.{toml,lock}), and the words of each word that is itself a
    command line (sh -c '...')."""
    words = []
    for token in tokens(text.replace("\\\n", " ")):
        if set(token) <= OPERATOR_CHARS:
            continue
        token = shown(token)
        words.append(token)
        words += [word for word in expand_braces(token) if word != token]
        if depth < MAX_DEPTH and re.search(r"[\s'\"\\]", token):
            words += command_words(token, depth + 1)
    return words


def expand_braces(word):
    """The words a shell brace expansion makes of `word` (a{b,c}d gives abd
    and acd), or `word` itself when it has none. A ${...} is no brace
    expansion, and neither is a {} without a comma in it. An expansion that
    makes more than BRACE_LIMIT words is denied: the guard would miss the
    words past the limit."""
    words = []
    expand_braces_into(word, words)
    return words


def expand_braces_into(word, words):
    depth, start = 0, None
    for index, character in enumerate(word):
        if character == "{" and not (index and word[index - 1] == "$"):
            if depth == 0:
                start, commas = index, []
            depth += 1
        elif character == "}" and depth:
            depth -= 1
            if depth == 0 and commas:
                last = start + 1
                for comma in commas + [index]:
                    expand_braces_into(word[:start] + word[last:comma] + word[index + 1:], words)
                    last = comma + 1
                return
        elif character == "," and depth == 1:
            commas.append(index)
    words.append(word)
    if len(words) > BRACE_LIMIT:
        block("a brace expansion in the command makes more words than the guard can check")


def names_key_file(command, directories):
    """Why a Bash command may not run, if it names a provider key file or a
    roko config file that holds a secret (see the top), else None. A word is
    resolved against each of `directories`, those the call may run in."""
    words = command_words(command)
    if any(KEY_PATH_TEXT.search(text) for text in [command] + words):
        return KEY_FILE_REASON
    if any(ROKO_DIR_WORD.search(os.path.normpath(word)) for word in words) and any(
        KEY_NAME_WORD.search(word) for word in words
    ):
        return KEY_FILE_REASON
    for word in words:
        for cwd in directories:
            # The word, a glob's matches as the shell would expand it (cat *),
            # and an option's or assignment's value (--env-file=x).
            paths = [] if not word or re.search(r"[$`{]", word) else expand(word, cwd)
            value = word.split("=", 1)[-1]
            if value != word and value and not re.search(r"[$`*?\[{]", value):
                paths.append(value)
            for path in paths:
                if is_key_path(path, cwd, False):
                    return KEY_FILE_REASON
                if is_secret_config_path(path, cwd):
                    return CONFIG_SECRET_REASON
    return None


def expand(word, cwd):
    """The paths a word names, resolved against cwd: a glob's matches (the
    shell expands a whole word, so --exclude=*.toml matches nothing), or the
    word itself."""
    path = os.path.join(cwd, os.path.expanduser(word))
    return glob.glob(path) if re.search(r"[*?\[]", word) else [path]


def is_key_path(path, cwd, search_root):
    """Whether a tool's path argument is a key file, as given or with
    symlinks resolved. A search root is also denied when it is a .roko
    directory, since the search would read the key files in it."""
    path = os.path.join(cwd, os.path.expanduser(path))
    for candidate in {os.path.normpath(path), os.path.realpath(path)}:
        parent, name = os.path.split(candidate)
        if os.path.basename(parent) == ".roko" and name in KEY_FILE_NAMES:
            return True
        if search_root and name == ".roko":
            return True
    return False


def is_secret_config_path(path, cwd):
    """Whether a path argument is a roko config file that holds a secret,
    as given or with symlinks resolved."""
    path = os.path.join(cwd, os.path.expanduser(path))
    return any(config_holds_secret(candidate) for candidate in {path, os.path.realpath(path)})


def sensitive_files(top, cwd):
    """The files in the tree at `top`, a real directory, that agents must
    not read, as (path, reason) pairs, of those the guard can find without
    walking the whole tree:
    - the roko config files that hold a secret, of those roko reads: the
      roko.toml in top, the workspace's (in cwd or above it, as roko finds
      it), the file ROKO_CONFIG names and the legacy
      ~/.config/roko/config.toml;
    - the key files in the .roko directories of key_directories."""
    inside = top.rstrip("/") + "/"
    found = []
    xdg = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    configs = [
        os.path.join(top, "roko.toml"), workspace_config(cwd), os.environ.get("ROKO_CONFIG"),
        os.path.join(xdg, "roko", "config.toml"),
    ]
    for config in configs:
        if config:
            directory, name = os.path.split(config)
            config = os.path.join(os.path.realpath(directory), name)
            if config.startswith(inside) and config_holds_secret(config):
                found.append((config, CONFIG_SECRET_REASON))
    for directory in key_directories(top, cwd):
        is_roko = os.path.basename(directory) == ".roko"
        roko = directory if is_roko else os.path.join(directory, ".roko")
        if (roko + "/").startswith(inside) and os.path.isdir(roko):
            found += [
                (os.path.join(roko, name), KEY_TREE_REASON)
                for name in KEY_FILE_NAMES
                if os.path.lexists(os.path.join(roko, name))
            ]
    return found


def key_directories(top, cwd):
    """The directories whose .roko holds key files a read of the tree at
    `top` may reach: top, its subdirectories two levels down (not under a
    hidden one, at most KEY_SEARCH_DIRS a level), cwd, the directory the
    Bash call runs in, their ancestors, and HOME."""
    directories, level = [top], [top]
    for _ in range(2):
        below = []
        for directory in level:
            if len(below) >= KEY_SEARCH_DIRS:
                break
            if directory != top and os.path.basename(directory).startswith("."):
                continue
            try:
                with os.scandir(directory) as entries:
                    below += [
                        entry.path for entry in entries if entry.is_dir(follow_symlinks=False)
                    ]
            except OSError:
                continue
        level = below[:KEY_SEARCH_DIRS]
        directories += level
    for start in (cwd, BASH_CALL["cwd"]):
        current = os.path.realpath(start) if start else "/"
        while True:
            directories.append(current)
            parent = os.path.dirname(current)
            if parent == current:
                break
            current = parent
    home = os.path.expanduser("~")
    if home != "~":
        directories.append(os.path.realpath(home))
    return directories


def workspace_config(cwd):
    """The roko.toml that roko loads in cwd: the first in cwd or above it."""
    directory = os.path.realpath(cwd)
    while True:
        config = os.path.join(directory, "roko.toml")
        if os.path.isfile(config):
            return config
        parent = os.path.dirname(directory)
        if parent == directory:
            return None
        directory = parent


def config_holds_secret(path):
    """Whether `path` is a roko config file outside .roko (roko.toml, the
    file ROKO_CONFIG names, the legacy ~/.config/roko/config.toml) holding a
    secret: a value roko config show would redact, as
    roko_core::child_env::is_config_with_secrets decides. With no TOML parser
    in python 3.9, the file is read line by line."""
    name = os.path.basename(path)
    roko_config = os.environ.get("ROKO_CONFIG")
    if not (
        name == "roko.toml"
        or (name == "config.toml" and os.path.basename(os.path.dirname(path)) == "roko")
        or (roko_config and os.path.realpath(roko_config) == os.path.realpath(path))
    ):
        return False
    # Only a regular file is read: opening a FIFO named roko.toml would block.
    if not os.path.isfile(path):
        return False
    try:
        with open(path, encoding="utf-8") as config:
            lines = config.read().splitlines()
    except (OSError, ValueError):
        return False
    table = ""
    for line in lines:
        if line.lstrip().startswith("#"):
            continue
        header = CONFIG_TABLE.match(line)
        if header:
            table = header.group(1).strip("\"'")
            continue
        headers = table.rsplit(".", 1)[-1] == "extra_headers"
        pairs = config_strings(INLINE_HEADERS.sub("", line))
        if any(secret_config_pair(key, value, headers) for key, value in pairs):
            return True
        for inline in INLINE_HEADERS.findall(line):
            if any(secret_config_pair(key, value, True) for key, value in config_strings(inline)):
                return True
        if table == "agent" and re.match(r"\s*env\s*=", line):
            for name, value in ENV_PAIR.findall(line):
                if SECRET_ENV_SEGMENTS & set(name.upper().split("_")) and literal_secret(value):
                    return True
    return False


def config_strings(line):
    """The (key, string value) pairs on a line of TOML."""
    return [(key, double or single) for key, double, single in CONFIG_STRING.findall(line)]


def secret_config_pair(key, value, headers):
    """Whether `key = value` holds a secret: a literal in a secret-named
    field or, other than a *_file path, in extra_headers."""
    field = key.rsplit(".", 1)[-1].lower()
    if headers:
        return not field.endswith("_file") and literal_secret(value)
    return bool(SECRET_CONFIG_KEY.search(field)) and not field.endswith("_env") and literal_secret(value)


def literal_secret(value):
    """Whether a config string is a literal: not empty, and no ${VAR} reference."""
    return bool(value) and "${" not in value


def greps_secret_config(tool_input, cwd):
    """Whether a Grep reads a roko config file that holds a secret, in the
    tree it searches (sensitive_files), unless its glob or type leaves the
    file out."""
    root = os.path.join(cwd, os.path.expanduser(tool_input.get("path") or "."))
    kind, pattern = tool_input.get("type"), tool_input.get("glob")
    if not os.path.isdir(root):
        return False
    top = os.path.realpath(root)
    return any(
        reason == CONFIG_SECRET_REASON
        and not (kind and kind not in file_types(os.path.basename(found)) | {"all"})
        and not (
            isinstance(pattern, str) and not glob_includes(pattern, os.path.relpath(found, top))
        )
        for found, reason in sensitive_files(top, cwd)
    )


def hook_cwd(data):
    """The directory the tool call runs in."""
    cwd = data.get("cwd")
    return cwd if isinstance(cwd, str) and cwd else os.getcwd()


def check_bash(tool_input, data):
    command = tool_input.get("command", data.get("command"))
    if not isinstance(command, str):
        block("the Bash command is not a string")
    cwd = hook_cwd(data)
    directories = call_directories(command, cwd)
    reason = names_key_file(command, directories)
    if reason:
        block(reason)
    BASH_CALL.update(cwd=cwd, dir=cwd, dirs=directories, command=command)
    check_command(command)


def check_file(tool_input, data):
    cwd = hook_cwd(data)
    # Read, Edit and Write take file_path, NotebookEdit notebook_path, and
    # Grep and Glob path, the directory they search.
    for field in ("file_path", "notebook_path", "path"):
        value = tool_input.get(field)
        if value is None:
            continue
        if not isinstance(value, str):
            block("the " + field + " argument is not a string")
        if value and is_key_path(value, cwd, search_root=field == "path"):
            block(KEY_FILE_REASON)
        if value and is_secret_config_path(value, cwd):
            block(CONFIG_SECRET_REASON)
    if data.get("tool_name") == "Grep" and greps_secret_config(tool_input, cwd):
        block(CONFIG_SECRET_REASON)


def main():
    check = sys.argv[1] if len(sys.argv) > 1 else "bash"
    try:
        data = json.load(sys.stdin)
    except ValueError:
        block("the hook input is not JSON")
    if not isinstance(data, dict):
        block("the hook input is not a JSON object")
    tool_input = data.get("tool_input") or data.get("toolInput") or {}
    if not isinstance(tool_input, dict):
        block("the hook input has no tool_input object")
    if check == "bash":
        check_bash(tool_input, data)
    elif check == "file":
        check_file(tool_input, data)
    else:
        block("unknown guard check " + repr(check))


try:
    main()
except SystemExit:
    raise
except Exception as error:
    block("the command guard failed: %r" % (error,))
sys.exit(0)
