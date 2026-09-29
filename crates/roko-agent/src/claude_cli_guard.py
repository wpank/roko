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
# - a word, resolved against the working directory with symlinks followed,
#   is a key file.
#
# The destructive commands are:
#
# - git checkout, switch, restore and push;
# - git branch -m, -M and -D (renames and force deletes);
# - git reset --hard;
# - git stash, except stash list and stash show;
# - git clean, except dry runs (-n);
# - recursive rm (-r, -R or --recursive, anywhere before --);
# - find -delete, and any rm that find -exec runs.
#
# Every command in a chain is checked (;, &&, ||, |, & and newlines), after
# assignments, shell keywords and wrappers such as sudo, env and xargs, and
# past git's global options (-C, -c, --git-dir, --work-tree). Commands run by
# subshells, sh -c, eval, $(...), backquotes, find -exec, busybox applets
# and command strings handed to wrappers (watch '...', flock -c '...') are
# checked too.
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
# Exit 0 lets the call run. Exit 2 blocks it, and Claude Code shows stderr
# to the model. Claude Code treats any other exit code as a non-blocking
# error and runs the call anyway, so every failure here, including
# unreadable input, exits 2.
#
# This is best effort, not a sandbox: a script file or a variable can still
# hide a command or a path from it.

import functools
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
    "su", "runuser", "script", "sg",
}
# Wrappers that hand every argument to sh -c as one command (watch 'rm -rf x').
STRING_WRAPPERS = {"watch", "sg"}
# Wrapper options whose value is a shell command (flock -c, su -c, env -S).
COMMAND_OPTIONS = {"-c", "--command", "-S", "--split-string"}
# Wrapper options whose value is a user or group, never the program (sudo -u git).
USER_OPTIONS = {"-u", "-g", "-U", "--user", "--group", "--other-user"}
# Multi-call binaries whose first argument names the program (busybox rm).
MULTICALL = {"busybox", "toybox"}
# find actions that run a command, up to ; or +.
FIND_EXEC = {"-exec", "-execdir", "-ok", "-okdir"}
# Programs that run their arguments as shell commands.
SHELLS = {"sh", "bash", "zsh", "dash", "ksh", "fish"}
# Programs whose arguments this guard checks.
CHECKED_PROGRAMS = {"git", "rm", "eval", "find"} | SHELLS | MULTICALL
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
# aliases are looked up in, and the whole command. under_find is set while
# the command a find -exec runs is checked.
BASH_CALL = {"cwd": None, "command": "", "under_find": False}


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
    """The word lists of the commands in `text`, split at its operators."""
    commands, words = [], []
    for token in tokens(text.replace("\\\n", " ")):
        characters = set(token)
        if characters and characters <= OPERATOR_CHARS and characters & SEPARATOR_CHARS:
            if words:
                commands.append(words)
            words = []
        else:
            words.append(token)
    if words:
        commands.append(words)
    return commands


def check_command(text, depth=0):
    if depth > MAX_DEPTH:
        block("the command nests shells too deeply to check")
    for match in SUBSTITUTION.finditer(text):
        inner = match.group(1) if match.group(1) is not None else match.group(2)
        check_command(inner, depth + 1)
    for words in simple_commands(text):
        check_words(words, depth)


def program_name(word):
    return os.path.basename(word)


def check_words(words, depth, maybe_argument=False):
    """Check one command. `maybe_argument`: its first word may be an
    argument of a wrapper rather than the program the wrapper runs."""
    while words and (words[0] in SHELL_KEYWORDS or ASSIGNMENT.match(words[0])):
        words = words[1:]
    if not words:
        return
    program = program_name(words[0])
    if program in MULTICALL:
        check_words(words[1:], depth, maybe_argument)
    elif program in WRAPPERS:
        check_wrapped(program, words, depth)
    elif program == "git":
        check_git(words[1:], depth, maybe_argument)
    elif program == "rm":
        check_rm(words[1:])
    elif program == "find":
        check_find(words[1:], depth)
    elif program in SHELLS:
        for argument in words[1:]:
            if not argument.startswith("-"):
                check_command(argument, depth + 1)
    elif program == "eval":
        check_command(" ".join(words[1:]), depth + 1)


def check_wrapped(program, words, depth):
    """Check the commands a wrapper (`words[0]`, named `program`) runs.

    Its own options come first, and an option's value can name a program too
    (xargs -a git rm -rf x), so every later word that names a program this
    guard checks starts a command to check. A user or group is skipped
    (sudo -u git whoami). After a word that is not an option, an assignment,
    a number (timeout's duration), a user or another wrapper, the word may
    be an argument instead (timeout 5 grep git src). A command handed over
    as one string (watch 'rm -rf x', flock l -c '...') is checked as a
    command line."""
    after_argument = False
    for index in range(1, len(words)):
        word, previous = words[index], words[index - 1]
        is_user = previous in USER_OPTIONS
        if program_name(word) in CHECKED_PROGRAMS and not is_user:
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
            under_find, BASH_CALL["under_find"] = BASH_CALL["under_find"], True
            check_words(command, depth)
            BASH_CALL["under_find"] = under_find


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
    if BASH_CALL["under_find"]:
        block("find -exec rm forbidden: it deletes files across the whole tree find walks")
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
    """Whether a Bash command names a provider key file (see the top)."""
    words = command_words(command)
    if any(KEY_PATH_TEXT.search(text) for text in [command] + words):
        return True
    if any(ROKO_DIR_WORD.search(os.path.normpath(word)) for word in words) and any(
        KEY_NAME_WORD.search(word) for word in words
    ):
        return True
    for word in words:
        # The word, and an option's or assignment's value (--env-file=x).
        for value in {word, word.split("=", 1)[-1]}:
            if value and not re.search(r"[$`*?\[{]", value) and is_key_path(value, cwd, False):
                return True
    return False


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


def hook_cwd(data):
    """The directory the tool call runs in."""
    cwd = data.get("cwd")
    return cwd if isinstance(cwd, str) and cwd else os.getcwd()


def check_bash(tool_input, data):
    command = tool_input.get("command", data.get("command"))
    if not isinstance(command, str):
        block("the Bash command is not a string")
    cwd = hook_cwd(data)
    if names_key_file(command, cwd):
        block(KEY_FILE_REASON)
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
