const vscode = require('vscode');
const { spawn } = require('child_process');

class LspClient {
  constructor(serverPath) {
    this.serverPath = serverPath;
    this.proc = null;
    this.nextId = 1;
    this.pending = new Map();
    this.buffer = Buffer.alloc(0);
    this.diagnostics = vscode.languages.createDiagnosticCollection('typephp');
  }

  start() {
    this.proc = spawn(this.serverPath, ['--lsp'], { stdio: ['pipe', 'pipe', 'pipe'] });
    this.proc.stdout.on('data', (chunk) => this.onData(chunk));
    this.proc.on('error', (err) => {
      vscode.window.showWarningMessage(`TypePHP LSP (${this.serverPath}) failed: ${err.message}`);
    });
    return this.request('initialize', { capabilities: {} });
  }

  stop() {
    if (this.proc) {
      this.notify('shutdown', {});
      this.notify('exit', {});
      this.proc.kill();
      this.proc = null;
    }
    this.diagnostics.dispose();
  }

  onData(chunk) {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    while (true) {
      const sepIdx = this.buffer.indexOf('\r\n\r\n');
      if (sepIdx === -1) break;
      const header = this.buffer.slice(0, sepIdx).toString('utf8');
      const match = /Content-Length:\s*(\d+)/i.exec(header);
      if (!match) {
        this.buffer = this.buffer.slice(sepIdx + 4);
        continue;
      }
      const len = parseInt(match[1], 10);
      const total = sepIdx + 4 + len;
      if (this.buffer.length < total) break;
      const body = this.buffer.slice(sepIdx + 4, total).toString('utf8');
      this.buffer = this.buffer.slice(total);
      try {
        this.handleMessage(JSON.parse(body));
      } catch (_) {}
    }
  }

  handleMessage(msg) {
    if (msg.id !== undefined && this.pending.has(msg.id)) {
      const resolve = this.pending.get(msg.id);
      this.pending.delete(msg.id);
      resolve(msg.result);
    } else if (msg.method === 'textDocument/publishDiagnostics' && msg.params) {
      const uri = vscode.Uri.parse(msg.params.uri);
      const list = (msg.params.diagnostics || []).map((d) => {
        const range = new vscode.Range(
          d.range.start.line,
          d.range.start.character,
          d.range.end.line,
          d.range.end.character
        );
        const sev = d.severity === 2
          ? vscode.DiagnosticSeverity.Warning
          : vscode.DiagnosticSeverity.Error;
        const diag = new vscode.Diagnostic(range, d.message, sev);
        diag.source = d.source || 'typephp';
        return diag;
      });
      this.diagnostics.set(uri, list);
    }
  }

  send(payload) {
    if (!this.proc || !this.proc.stdin.writable) return;
    const json = JSON.stringify(payload);
    const frame = `Content-Length: ${Buffer.byteLength(json, 'utf8')}\r\n\r\n${json}`;
    this.proc.stdin.write(frame);
  }

  notify(method, params) {
    this.send({ jsonrpc: '2.0', method, params });
  }

  request(method, params) {
    const id = this.nextId++;
    return new Promise((resolve) => {
      this.pending.set(id, resolve);
      this.send({ jsonrpc: '2.0', id, method, params });
    });
  }
}

function toVscodeLocation(loc) {
  if (!loc || !loc.uri || !loc.range) return null;
  const range = new vscode.Range(
    loc.range.start.line,
    loc.range.start.character,
    loc.range.end.line,
    loc.range.end.character
  );
  return new vscode.Location(vscode.Uri.parse(loc.uri), range);
}

function activate(context) {
  const cfg = vscode.workspace.getConfiguration('typephp');
  const serverPath = cfg.get('serverPath') || 'tphpc';
  const client = new LspClient(serverPath);
  client.start();

  const syncDoc = (doc) => {
    if (doc.languageId !== 'typephp') return;
    client.notify('textDocument/didOpen', {
      textDocument: { uri: doc.uri.toString(), languageId: 'typephp', version: doc.version, text: doc.getText() }
    });
  };

  vscode.workspace.textDocuments.forEach(syncDoc);
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(syncDoc),
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (e.document.languageId !== 'typephp') return;
      client.notify('textDocument/didChange', {
        textDocument: { uri: e.document.uri.toString(), version: e.document.version },
        contentChanges: [{ text: e.document.getText() }]
      });
    }),
    vscode.languages.registerHoverProvider('typephp', {
      async provideHover(doc, pos) {
        const res = await client.request('textDocument/hover', {
          textDocument: { uri: doc.uri.toString() },
          position: { line: pos.line, character: pos.character }
        });
        if (!res || !res.contents) return null;
        return new vscode.Hover(new vscode.MarkdownString(res.contents.value));
      }
    }),
    vscode.languages.registerDefinitionProvider('typephp', {
      async provideDefinition(doc, pos) {
        const res = await client.request('textDocument/definition', {
          textDocument: { uri: doc.uri.toString() },
          position: { line: pos.line, character: pos.character }
        });
        if (Array.isArray(res)) {
          return res.map(toVscodeLocation).filter(Boolean);
        }
        return toVscodeLocation(res);
      }
    }),
    vscode.languages.registerReferenceProvider('typephp', {
      async provideReferences(doc, pos) {
        const res = await client.request('textDocument/references', {
          textDocument: { uri: doc.uri.toString() },
          position: { line: pos.line, character: pos.character }
        });
        if (!Array.isArray(res)) return [];
        return res.map(toVscodeLocation).filter(Boolean);
      }
    }),
    { dispose: () => client.stop() }
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
