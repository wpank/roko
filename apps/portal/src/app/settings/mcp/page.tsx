'use client';

import { useState } from 'react';
import { clsx } from 'clsx';
import { Plus, Trash2, Settings2, TestTube, Terminal, Globe } from 'lucide-react';
import { Button, Badge, StatusLED } from '@/components/atoms';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type McpTransport = 'stdio' | 'sse';
type McpStatus = 'active' | 'inactive' | 'error';

interface McpServer {
  id: string;
  name: string;
  transport: McpTransport;
  status: McpStatus;
  toolCount: number;
  command?: string;
  url?: string;
  description: string;
}

// ---------------------------------------------------------------------------
// Placeholder data
// ---------------------------------------------------------------------------

const INITIAL_SERVERS: McpServer[] = [
  {
    id: 'roko-mcp-code',
    name: 'roko-mcp-code',
    transport: 'stdio',
    status: 'active',
    toolCount: 12,
    command: 'roko-mcp-code --workspace .',
    description: 'Code-intelligence MCP — AST search, symbol graph, diff analysis.',
  },
  {
    id: 'roko-mcp-github',
    name: 'roko-mcp-github',
    transport: 'stdio',
    status: 'active',
    toolCount: 8,
    command: 'roko-mcp-github',
    description: 'GitHub MCP — PRs, issues, workflow status, comments.',
  },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function statusToLed(s: McpStatus): 'success' | 'error' | 'offline' {
  if (s === 'active') return 'success';
  if (s === 'error') return 'error';
  return 'offline';
}

// ---------------------------------------------------------------------------
// ServerRow
// ---------------------------------------------------------------------------

interface ServerRowProps {
  server: McpServer;
  onRemove: (id: string) => void;
  onTest: (id: string) => void;
  testing: boolean;
}

function ServerRow({ server, onRemove, onTest, testing }: ServerRowProps) {
  const TransportIcon = server.transport === 'stdio' ? Terminal : Globe;

  return (
    <div
      className={clsx(
        'flex items-start gap-4 px-4 py-3',
        'border-b border-b-[var(--text-ghost)] last:border-b-0',
        'hover:bg-[var(--bg-highlight)]',
        'transition-colors duration-[80ms]',
      )}
    >
      {/* Status LED */}
      <div className="mt-1 shrink-0">
        <StatusLED status={statusToLed(server.status)} pulse={server.status === 'active'} />
      </div>

      {/* Server info */}
      <div className="flex flex-col gap-1 flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)]">
            {server.name}
          </span>

          {/* Transport badge */}
          <span
            className={clsx(
              'inline-flex items-center gap-1',
              'font-[var(--font-mono)] text-[var(--text-xs)]',
              'px-1.5 py-0.5 border',
              'text-[var(--text-faint)] border-[var(--text-ghost)]',
            )}
          >
            <TransportIcon size={10} strokeWidth={1.5} aria-hidden />
            {server.transport}
          </span>

          {/* Tool count */}
          <Badge variant="info">{server.toolCount} tools</Badge>
        </div>

        <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-relaxed">
          {server.description}
        </p>

        {server.command && (
          <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] mt-0.5">
            $ {server.command}
          </code>
        )}
        {server.url && (
          <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] mt-0.5">
            {server.url}
          </code>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 shrink-0">
        <Button
          variant="ghost"
          size="sm"
          loading={testing}
          onClick={() => onTest(server.id)}
          title="Test MCP connectivity"
        >
          <TestTube size={12} strokeWidth={1.5} aria-hidden />
          Test
        </Button>
        <Button
          variant="ghost"
          size="sm"
          title="Configure server"
        >
          <Settings2 size={12} strokeWidth={1.5} aria-hidden />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onRemove(server.id)}
          title="Remove server"
          className="hover:text-[var(--accent-error)] hover:border-[var(--accent-error)]"
        >
          <Trash2 size={12} strokeWidth={1.5} aria-hidden />
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Add-server form
// ---------------------------------------------------------------------------

interface AddServerFormProps {
  onAdd: (name: string, transport: McpTransport, command: string) => void;
  adding: boolean;
}

function AddServerForm({ onAdd, adding }: AddServerFormProps) {
  const [name, setName] = useState('');
  const [transport, setTransport] = useState<McpTransport>('stdio');
  const [command, setCommand] = useState('');

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim() || !command.trim()) return;
    onAdd(name.trim(), transport, command.trim());
    setName('');
    setCommand('');
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div className="grid grid-cols-2 gap-4">
        {/* Name */}
        <div className="flex flex-col gap-1">
          <label
            htmlFor="mcp-name"
            className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
          >
            Server name
          </label>
          <input
            id="mcp-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="my-mcp-server"
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-sm)]',
              'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
              'border border-[var(--text-ghost)]',
              'px-2 py-1.5',
              'placeholder:text-[var(--text-ghost)]',
              'focus:border-[var(--border-active)]',
              'outline-none',
            )}
          />
        </div>

        {/* Transport */}
        <div className="flex flex-col gap-1">
          <label
            htmlFor="mcp-transport"
            className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
          >
            Transport
          </label>
          <select
            id="mcp-transport"
            value={transport}
            onChange={(e) => setTransport(e.target.value as McpTransport)}
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-sm)]',
              'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
              'border border-[var(--text-ghost)]',
              'px-2 py-1.5',
              'focus:border-[var(--border-active)]',
              'outline-none',
            )}
          >
            <option value="stdio">stdio</option>
            <option value="sse">sse</option>
          </select>
        </div>
      </div>

      {/* Command / URL */}
      <div className="flex flex-col gap-1">
        <label
          htmlFor="mcp-command"
          className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
        >
          {transport === 'stdio' ? 'Command' : 'Endpoint URL'}
        </label>
        <input
          id="mcp-command"
          type="text"
          value={command}
          onChange={(e) => setCommand(e.target.value)}
          placeholder={transport === 'stdio' ? 'my-server --arg value' : 'http://localhost:3100/sse'}
          className={clsx(
            'font-[var(--font-mono)] text-[var(--text-sm)]',
            'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
            'border border-[var(--text-ghost)]',
            'px-2 py-1.5',
            'placeholder:text-[var(--text-ghost)]',
            'focus:border-[var(--border-active)]',
            'outline-none',
          )}
        />
      </div>

      <div className="flex justify-end">
        <Button
          type="submit"
          variant="primary"
          size="sm"
          loading={adding}
          disabled={!name.trim() || !command.trim()}
        >
          <Plus size={12} strokeWidth={1.5} aria-hidden />
          Add server
        </Button>
      </div>
    </form>
  );
}

// ---------------------------------------------------------------------------
// MCP page
// ---------------------------------------------------------------------------

export default function McpPage() {
  const [servers, setServers] = useState<McpServer[]>(INITIAL_SERVERS);
  const [testingId, setTestingId] = useState<string | null>(null);

  function handleRemove(id: string) {
    setServers((prev) => prev.filter((s) => s.id !== id));
  }

  function handleTest(id: string) {
    setTestingId(id);
    setTimeout(() => setTestingId(null), 1500);
  }

  function handleAdd(name: string, transport: McpTransport, command: string) {
    const newServer: McpServer = {
      id: name,
      name,
      transport,
      status: 'inactive',
      toolCount: 0,
      command: transport === 'stdio' ? command : undefined,
      url: transport === 'sse' ? command : undefined,
      description: 'Newly added MCP server',
    };
    setServers((prev) => [...prev, newServer]);
  }

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Page header */}
      <div className="flex flex-col gap-1">
        <h1 className="font-[var(--font-mono)] text-[var(--text-lg)] text-[var(--text-strong)]">
          MCP Servers
        </h1>
        <p className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)]">
          Configure Model Context Protocol servers that agents can call.
        </p>
      </div>

      {/* Server list */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Active servers
        </h2>

        {servers.length === 0 ? (
          <div className="border border-[var(--text-ghost)] px-4 py-6 text-center">
            <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-faint)]">
              No MCP servers configured
            </span>
          </div>
        ) : (
          <div className="border border-[var(--text-ghost)]">
            {servers.map((server) => (
              <ServerRow
                key={server.id}
                server={server}
                onRemove={handleRemove}
                onTest={handleTest}
                testing={testingId === server.id}
              />
            ))}
          </div>
        )}
      </section>

      {/* Add server form */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Add MCP server
        </h2>
        <div className="border border-[var(--text-ghost)] p-4">
          <AddServerForm onAdd={handleAdd} adding={false} />
        </div>
      </section>
    </div>
  );
}
