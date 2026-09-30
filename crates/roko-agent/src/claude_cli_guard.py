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
# - a word, resolved against the working directory with symlinks followed
#   and a glob expanded (cat *), is a key file, or a roko config file that
#   holds a secret (see below).
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
# such as serve.auth.api_key, and so is a recursive search of a tree that
# holds one (a Grep, grep -r, rg), unless the search's glob or type leaves
# the file out. roko itself refuses to load such a file (the secret belongs
# in .roko/.env), so this matters for a secret added while roko runs. A
# search after cd is judged from the call's directory, and a read through
# find, xargs, git grep or a script is not caught.
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
    {"git", "rm", "unlink", "shred", "eval", "ssh"} | FINDERS | SHELLS | MULTICALL | GREPS | RIPGREPS
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
MAX_DEPTH = 8

# The Bash call being checked (check_bash sets it): the directory git
# aliases are looked up in, and the whole command. bulk is set, to why a
# delete may not run, while a command that runs on a list the guard cannot
# see is checked: what find or fd lists, or what xargs reads.
BASH_CALL = {"cwd": None, "command": "", "bulk": None}
FIND_BULK = "rm on what find or fd lists is forbidden: it deletes files across a whole tree"
XARGS_BULK = (
    "a delete run by xargs is forbidden: it deletes every file in a list the guard cannot see"
)
PARALLEL_BULK = "a delete run by parallel is forbidden: it deletes every file in its list"
# Wrappers that run their command on each item of a list.
BULK_RUNNERS = {"xargs": XARGS_BULK, "parallel": PARALLEL_BULK}


def tokens(text):
    lexer = shlex.shlex(text, posix=True, punctuation_chars="();<>|&\n")
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


def simple_commands(text):
    """The commands in `text`, split at its operators, as (words, fed)
    pairs: fed when a pipe brings the command what find or fd lists,
    directly or into a loop (find . | while read f; do rm "$f"; done)."""
    commands, words = [], []
    finds = fed = False
    blocks = 0  # compound commands opened since the pipe
    for token in tokens(text.replace("\\\n", " ")):
        characters = set(token)
        if not (characters and characters <= OPERATOR_CHARS and characters & SEPARATOR_CHARS):
            if fed and not command_start(words):
                if token in BLOCK_OPENERS:
                    blocks += 1
                elif token in BLOCK_CLOSERS and blocks:
                    blocks -= 1
            words.append(token)
            continue
        if words:
            commands.append((words, fed))
            finds = finds or runs_finder(words)
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
            finds = fed = False
    if words:
        commands.append((words, fed))
    return commands


def check_command(text, depth=0):
    if depth > MAX_DEPTH:
        block("the command nests shells too deeply to check")
    # A substitution can hand a command what find lists (rm $(find . -name x)).
    listed = False
    for match in SUBSTITUTION.finditer(text):
        inner = match.group(1) if match.group(1) is not None else match.group(2)
        check_command(inner, depth + 1)
        listed = listed or any(runs_finder(words) for words, _ in simple_commands(inner))
    for words, fed in simple_commands(text):
        if fed or listed:
            check_found(words, depth)
        else:
            check_words(words, depth)


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
    elif program in GREPS or program in RIPGREPS:
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
def bulk(reason):
    """While the block checks a command that runs on a list the guard cannot
    see, deny any delete in it, for `reason`; None changes nothing."""
    previous = BASH_CALL["bulk"]
    BASH_CALL["bulk"] = previous or reason
    try:
        yield
    finally:
        BASH_CALL["bulk"] = previous


def check_found(command, depth, reason=FIND_BULK, maybe_argument=False):
    """Check a command that runs on a list the guard cannot see: what find
    or fd lists (find -exec, fd -x, find . | xargs). Any delete in it is
    denied, for `reason`."""
    with bulk(reason):
        check_words(command, depth, maybe_argument)


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
            check_found(command, depth)


def check_fd(arguments, depth):
    # fd -x runs every word after it, up to a ;, on each result.
    for index, argument in enumerate(arguments):
        option, _, value = argument.partition("=")
        if option in FD_EXEC:
            check_found(([value] if value else []) + arguments[index + 1:], depth)
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
    """Deny a recursive search (grep -r, rg) that would read a roko config
    file holding a secret, unless its filters leave the file out."""
    ripgrep = program in RIPGREPS
    options, operands = search_arguments(
        arguments,
        RG_SHORT_VALUES if ripgrep else GREP_SHORT_VALUES,
        RG_LONG_VALUES if ripgrep else GREP_LONG_VALUES,
    )
    names = {name for name, _ in options}
    if ripgrep:
        if names & {"--files", "--type-list"}:
            return  # lists names, reads no file
    elif program != "rgrep" and not (
        names & {"-r", "-R", "--recursive", "--dereference-recursive"}
        or any(name in ("-d", "--directories") and value == "recurse" for name, value in options)
    ):
        return  # a file it names is checked with the other words
    if not names & {"-e", "-f", "--regexp", "--file"}:
        operands = operands[1:]  # the first is the pattern
    if search_skips_config(options, ripgrep):
        return
    cwd = BASH_CALL["cwd"] or os.getcwd()
    for operand in operands or ["."]:
        if re.search(r"[$`{]", operand):
            continue
        if any(reads_secret_config(path, cwd) for path in expand(operand, cwd)):
            block(CONFIG_SECRET_REASON)


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


def search_skips_config(options, ripgrep):
    """Whether a search's filters leave roko.toml out: grep's --include and
    --exclude, rg's -g (! excludes), -t and -T."""
    includes, excludes, types, skipped_types = [], [], set(), set()
    for name, value in options:
        if name == "--include":
            includes.append(value)
        elif name == "--exclude":
            excludes.append(value)
        elif ripgrep and name in ("-g", "--glob", "--iglob"):
            if value.startswith("!"):
                excludes.append(value[1:])
            else:
                includes.append(value)
        elif ripgrep and name in ("-t", "--type"):
            types.add(value)
        elif ripgrep and name in ("-T", "--type-not"):
            skipped_types.add(value)
    return bool(
        (includes and not any(glob_matches_config(pattern, False) for pattern in includes))
        or any(glob_matches_config(pattern, True) for pattern in excludes)
        or (types and not types & {"toml", "all"})
        or "toml" in skipped_types
    )


def glob_matches_config(pattern, strict):
    """Whether a file glob (grep --include, rg -g) matches roko.toml. Unless
    `strict`, a glob that names toml counts too, as a brace glob may
    (*.{rs,toml})."""
    pattern = pattern.lower().lstrip("/")
    while pattern.startswith("**/"):
        pattern = pattern[3:]
    return fnmatch.fnmatch("roko.toml", pattern) or (not strict and "toml" in pattern)


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
    """The words of `text` with quotes removed, and the words of each word
    that is itself a command line (sh -c '...')."""
    words = []
    for token in tokens(text.replace("\\\n", " ")):
        if set(token) <= OPERATOR_CHARS:
            continue
        words.append(token)
        if depth < MAX_DEPTH and re.search(r"[\s'\"\\]", token):
            words += command_words(token, depth + 1)
    return words


def names_key_file(command, cwd):
    """Why a Bash command may not run, if it names a provider key file or a
    roko config file that holds a secret (see the top), else None."""
    words = command_words(command)
    if any(KEY_PATH_TEXT.search(text) for text in [command] + words):
        return KEY_FILE_REASON
    if any(ROKO_DIR_WORD.search(os.path.normpath(word)) for word in words) and any(
        KEY_NAME_WORD.search(word) for word in words
    ):
        return KEY_FILE_REASON
    for word in words:
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


def reads_secret_config(path, cwd):
    """Whether a recursive search of `path` reads a roko config file that
    holds a secret: path itself, or one in the tree under it, of those roko
    reads: the roko.toml in path, the workspace's (in cwd or above it, as
    roko finds it), the file ROKO_CONFIG names and the legacy
    ~/.config/roko/config.toml."""
    if not os.path.isdir(path):
        return is_secret_config_path(path, cwd)
    root = os.path.realpath(path)
    xdg = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    configs = {
        os.path.join(root, "roko.toml"), workspace_config(cwd), os.environ.get("ROKO_CONFIG"),
        os.path.join(xdg, "roko", "config.toml"),
    }
    return any(
        config and os.path.commonpath([os.path.realpath(os.path.dirname(config)), root]) == root
        and config_holds_secret(config)
        for config in configs
    )


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
    tree it searches (reads_secret_config), unless its glob or type leaves
    the file out."""
    root = os.path.join(cwd, os.path.expanduser(tool_input.get("path") or "."))
    kind, pattern = tool_input.get("type"), tool_input.get("glob")
    if kind and kind != "toml":
        return False
    if isinstance(pattern, str) and not glob_matches_config(pattern, False):
        return False
    return os.path.isdir(root) and reads_secret_config(root, cwd)


def hook_cwd(data):
    """The directory the tool call runs in."""
    cwd = data.get("cwd")
    return cwd if isinstance(cwd, str) and cwd else os.getcwd()


def check_bash(tool_input, data):
    command = tool_input.get("command", data.get("command"))
    if not isinstance(command, str):
        block("the Bash command is not a string")
    cwd = hook_cwd(data)
    reason = names_key_file(command, cwd)
    if reason:
        block(reason)
    BASH_CALL.update(cwd=cwd, command=command)
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
