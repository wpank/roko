# PreToolUse guard for Claude CLI agents.
#
# build_settings_json (claude_cli_agent.rs) writes this script into the
# Claude CLI `--settings` payload, which runs it with `python3 -c` before
# every Bash call and passes the hook input as JSON on stdin. It denies
# commands that discard the operator's work or move branches:
#
# - git checkout, switch, restore and push;
# - git branch -m, -M and -D (renames and force deletes);
# - git reset --hard;
# - git stash, except stash list and stash show;
# - git clean, except dry runs (-n);
# - recursive rm.
#
# Every command in a chain is checked (;, &&, ||, |, & and newlines), after
# assignments, shell keywords and wrappers such as sudo, env and xargs, and
# past git's global options (-C, -c, --git-dir, --work-tree). Commands run by
# sh -c, eval, $(...) and backquotes are checked too.
#
# Exit 0 lets the command run. Exit 2 blocks it, and Claude Code shows
# stderr to the model. Claude Code treats any other exit code as a
# non-blocking error and runs the command anyway, so every failure here,
# including unreadable input, exits 2.
#
# This is best effort, not a sandbox: a script file, a git alias or a
# variable can still hide a command from it.

import json
import os
import re
import shlex
import sys


def block(reason):
    sys.stderr.write("BLOCKED: " + reason + "\n")
    sys.exit(2)


# Recursive rm, matched on the raw command as before this guard parsed it.
RM_PATTERNS = [
    r"(^|[;&|]\s*)rm\s+-[A-Za-z]*r[A-Za-z]*f[A-Za-z]*(?:\s|$)",
    r"(^|[;&|]\s*)rm\s+-[A-Za-z]*f[A-Za-z]*r[A-Za-z]*(?:\s|$)",
    r"(^|[;&|]\s*)rm\s+-[A-Za-z]*r[A-Za-z]*(?:\s|$)",
]

# Words that can start a command without being its program.
SHELL_KEYWORDS = {"if", "then", "else", "elif", "do", "while", "until", "!", "{", "}", "time"}
# Programs that run another program named later in their arguments.
WRAPPERS = {
    "sudo", "doas", "env", "command", "builtin", "exec", "nohup", "nice", "ionice", "time",
    "timeout", "gtimeout", "xargs", "stdbuf", "unbuffer", "chronic", "caffeinate", "watch", "flock",
}
# Programs that run their arguments as shell commands.
SHELLS = {"sh", "bash", "zsh", "dash", "ksh", "fish"}
# git global options whose value is the next argument.
GIT_VALUE_OPTIONS = {
    "-C", "-c", "--git-dir", "--work-tree", "--namespace", "--super-prefix", "--config-env",
    "--attr-source",
}

ASSIGNMENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*=")
SUBSTITUTION = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")
# Operators that end a command. Redirections (<, >) do not.
SEPARATOR_CHARS = set(";&|()\n")
OPERATOR_CHARS = SEPARATOR_CHARS | set("<>")
FALLBACK_TOKEN = re.compile(r"[;&|()<>\n]+|[^\s;&|()<>]+")
MAX_DEPTH = 8


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


def check_words(words, depth):
    while words and (words[0] in SHELL_KEYWORDS or ASSIGNMENT.match(words[0])):
        words = words[1:]
    if not words:
        return
    program = program_name(words[0])
    if program in WRAPPERS:
        # The wrapper's own options come first; the wrapped program is the
        # first later word that names a program this guard checks.
        for index in range(1, len(words)):
            name = program_name(words[index])
            if name == "git" or name == "eval" or name in SHELLS:
                check_words(words[index:], depth)
                return
    elif program == "git":
        check_git(words[1:])
    elif program in SHELLS:
        for argument in words[1:]:
            if not argument.startswith("-"):
                check_command(argument, depth + 1)
    elif program == "eval":
        check_command(" ".join(words[1:]), depth + 1)


def short_flags(arguments):
    """The letters of the short-option clusters in `arguments` (-fdx gives f, d, x)."""
    letters = set()
    for argument in arguments:
        if argument == "--":
            break
        if len(argument) > 1 and argument[0] == "-" and argument[1] != "-":
            letters.update(argument[1:])
    return letters


def check_git(arguments):
    index = 0
    while index < len(arguments) and arguments[index].startswith("-"):
        index += 2 if arguments[index] in GIT_VALUE_OPTIONS else 1
    if index >= len(arguments):
        return
    subcommand, rest = arguments[index], arguments[index + 1:]
    flags = short_flags(rest)
    if "$" in subcommand or "`" in subcommand:
        block("a git subcommand taken from a variable cannot be checked")
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


def main():
    try:
        data = json.load(sys.stdin)
    except ValueError:
        block("the hook input is not JSON")
    if not isinstance(data, dict):
        block("the hook input is not a JSON object")
    tool_input = data.get("tool_input") or data.get("toolInput") or {}
    if not isinstance(tool_input, dict):
        block("the hook input has no tool_input object")
    command = tool_input.get("command", data.get("command"))
    if not isinstance(command, str):
        block("the Bash command is not a string")
    for pattern in RM_PATTERNS:
        if re.search(pattern, command):
            block("destructive file deletion forbidden")
    check_command(command)


try:
    main()
except SystemExit:
    raise
except Exception as error:
    block("the command guard failed: %r" % (error,))
sys.exit(0)
