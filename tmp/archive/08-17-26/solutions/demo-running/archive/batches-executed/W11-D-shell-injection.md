# W11-D: Shell Injection in Demo Terminal Session

**Priority**: P0 -- shell metacharacters in model name allow arbitrary command injection
**Effort**: ~5 min
**Files to modify**: 1
**Dependencies**: None

## Problem

The `roko()` command builder in `terminal-session.ts` injects `ctx.activeModel` into shell commands without quoting:

```typescript
return `${bin} --model ${ctx.activeModel} ${subcommand}`;
```

If `activeModel` contains shell metacharacters (e.g., `"; rm -rf /; echo "`), the interpolated string becomes a shell injection vector. This is the demo app's terminal session, which executes commands in a real pty via `node-pty`.

A `shellQuote()` function already exists at line 104 of the same file and is used elsewhere (line 120 for `ensureWorkspaceCwd`), but is not used here.

## Exact Code to Change

### File 1: `demo/demo-app/src/lib/terminal-session.ts`

#### Change 1: Quote `activeModel` with `shellQuote()`

**Find this code** (line 139):
```typescript
export function roko(ctx: ScenarioContext, subcommand: string): string {
  const bin = getRoko();
  if (ctx.activeModel) {
    return `${bin} --model ${ctx.activeModel} ${subcommand}`;
  }
  return `${bin} ${subcommand}`;
}
```

**Replace with:**
```typescript
export function roko(ctx: ScenarioContext, subcommand: string): string {
  const bin = getRoko();
  if (ctx.activeModel) {
    return `${bin} --model ${shellQuote(ctx.activeModel)} ${subcommand}`;
  }
  return `${bin} ${subcommand}`;
}
```

Note: `shellQuote` is defined at line 104 in the same file as:
```typescript
function shellQuote(value: string): string {
  return `'${value.replace(/'/g, "'\\''")}'`;
}
```

It is already used at line 120 for `ensureWorkspaceCwd`. No import needed.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# 1. Verify the fix is in place
grep -n 'shellQuote.*activeModel' demo/demo-app/src/lib/terminal-session.ts
# Should show the quoted version

# 2. Verify no unquoted activeModel remains in command strings
grep -n 'ctx.activeModel' demo/demo-app/src/lib/terminal-session.ts
# The only interpolation into a command string should use shellQuote

# 3. Verify shellQuote is defined in the same file
grep -n 'function shellQuote' demo/demo-app/src/lib/terminal-session.ts
# Should show line 104

# 4. TypeScript type check (if available)
cd demo/demo-app && npx tsc --noEmit 2>&1 | head -20
```

## Agent Prompt

```
Fix shell injection vulnerability in `/Users/will/dev/nunchi/roko/roko/demo/demo-app/src/lib/terminal-session.ts`.

## Context

At line 139, the `roko()` function builds a shell command string for execution via node-pty.
`ctx.activeModel` is interpolated without quoting:
```typescript
return `${bin} --model ${ctx.activeModel} ${subcommand}`;
```

A `shellQuote()` function already exists at line 104 of the same file:
```typescript
function shellQuote(value: string): string {
  return `'${value.replace(/'/g, "'\\''")}'`;
}
```

It is already used at line 120 (`cd ${shellQuote(dir)}`).

## Fix

Change line 142 from:
```typescript
    return `${bin} --model ${ctx.activeModel} ${subcommand}`;
```
to:
```typescript
    return `${bin} --model ${shellQuote(ctx.activeModel)} ${subcommand}`;
```

This is a one-line fix. Verify with `grep -n 'ctx.activeModel' demo/demo-app/src/lib/terminal-session.ts`
to confirm no other unquoted interpolations exist in command strings.
```

## Commit

This batch is committed with Wave 11. Do not commit individually.

## Checklist

- [ ] `ctx.activeModel` wrapped in `shellQuote()` in the `roko()` function
- [ ] No other unquoted `ctx.activeModel` interpolations in command strings
- [ ] TypeScript type check passes (if project has tsconfig)

## Audit Status

Audited: 2026-05-05. PASS no changes needed.
