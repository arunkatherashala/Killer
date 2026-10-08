// Killer language client: launches `killer_lsp` over stdio and speaks LSP to it.
// Plain JS, no npm dependencies beyond the `vscode` API (a minimal JSON-RPC client is built in).
"use strict";

const cp = require("child_process");
const vscode = require("vscode");

let child = null;
let buffer = Buffer.alloc(0);
let nextId = 1;
const pending = new Map();
let diagnostics = null;
let output = null;

function send(msg) {
  if (!child) return;
  const body = Buffer.from(JSON.stringify(msg), "utf8");
  child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
  child.stdin.write(body);
}

function request(method, params) {
  return new Promise((resolve, reject) => {
    if (!child) { resolve(null); return; }
    const id = nextId++;
    pending.set(id, { resolve, reject });
    send({ jsonrpc: "2.0", id, method, params });
  });
}

function notify(method, params) {
  send({ jsonrpc: "2.0", method, params });
}

function onData(chunk) {
  buffer = Buffer.concat([buffer, chunk]);
  for (;;) {
    const headerEnd = buffer.indexOf("\r\n\r\n");
    if (headerEnd < 0) return;
    const header = buffer.slice(0, headerEnd).toString("ascii");
    const m = /Content-Length:\s*(\d+)/i.exec(header);
    if (!m) { buffer = buffer.slice(headerEnd + 4); continue; }
    const len = parseInt(m[1], 10);
    if (buffer.length < headerEnd + 4 + len) return;
    const body = buffer.slice(headerEnd + 4, headerEnd + 4 + len).toString("utf8");
    buffer = buffer.slice(headerEnd + 4 + len);
    let msg;
    try { msg = JSON.parse(body); } catch (e) { continue; }
    handle(msg);
  }
}

function handle(msg) {
  if (msg.id !== undefined && !msg.method) {
    const p = pending.get(msg.id);
    if (p) {
      pending.delete(msg.id);
      msg.error ? p.reject(new Error(msg.error.message)) : p.resolve(msg.result);
    }
    return;
  }
  if (msg.method === "textDocument/publishDiagnostics") {
    const uri = vscode.Uri.parse(msg.params.uri);
    const list = msg.params.diagnostics.map((d) => {
      const range = new vscode.Range(d.range.start.line, d.range.start.character, d.range.end.line, d.range.end.character);
      const sev = d.severity === 2 ? vscode.DiagnosticSeverity.Warning : vscode.DiagnosticSeverity.Error;
      const diag = new vscode.Diagnostic(range, d.message, sev);
      diag.source = d.source || "killer";
      return diag;
    });
    diagnostics.set(uri, list);
  } else if (msg.method === "window/logMessage" && output) {
    output.appendLine(msg.params.message);
  }
}

function pos(p) { return { line: p.line, character: p.character }; }
function toRange(r) { return new vscode.Range(r.start.line, r.start.character, r.end.line, r.end.character); }

function openDoc(doc) {
  if (doc.languageId !== "killer") return;
  notify("textDocument/didOpen", {
    textDocument: { uri: doc.uri.toString(), languageId: "killer", version: doc.version, text: doc.getText() },
  });
}

function activate(context) {
  output = vscode.window.createOutputChannel("Killer Language Server");
  diagnostics = vscode.languages.createDiagnosticCollection("killer");
  context.subscriptions.push(output, diagnostics);

  const cfg = vscode.workspace.getConfiguration("killer");
  const command = cfg.get("server.path") || "killer_lsp";
  const args = cfg.get("server.args") || [];
  try {
    child = cp.spawn(command, args, { stdio: ["pipe", "pipe", "pipe"] });
  } catch (e) {
    vscode.window.showErrorMessage("Killer: cannot start language server: " + e.message);
    return;
  }
  child.on("error", (e) => {
    vscode.window.showWarningMessage(`Killer: language server '${command}' failed to start (${e.message}). Set killer.server.path.`);
    child = null;
  });
  child.stdout.on("data", onData);
  child.stderr.on("data", (d) => output.append(d.toString()));

  const root = vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders[0];
  request("initialize", {
    processId: process.pid,
    rootUri: root ? root.uri.toString() : null,
    capabilities: {},
  }).then(() => {
    notify("initialized", {});
    vscode.workspace.textDocuments.forEach(openDoc);
  }).catch((e) => output.appendLine("initialize failed: " + e.message));

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(openDoc),
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (e.document.languageId !== "killer") return;
      notify("textDocument/didChange", {
        textDocument: { uri: e.document.uri.toString(), version: e.document.version },
        contentChanges: [{ text: e.document.getText() }],
      });
    }),
    vscode.workspace.onDidCloseTextDocument((doc) => {
      if (doc.languageId !== "killer") return;
      diagnostics.delete(doc.uri);
      notify("textDocument/didClose", { textDocument: { uri: doc.uri.toString() } });
    }),
    vscode.languages.registerCompletionItemProvider("killer", {
      provideCompletionItems(doc, position) {
        return request("textDocument/completion", {
          textDocument: { uri: doc.uri.toString() }, position: pos(position),
        }).then((res) => {
          const items = Array.isArray(res) ? res : (res && res.items) || [];
          return items.map((i) => {
            const it = new vscode.CompletionItem(i.label, i.kind ? i.kind - 1 : undefined);
            it.detail = i.detail;
            return it;
          });
        });
      },
    }),
    vscode.languages.registerHoverProvider("killer", {
      provideHover(doc, position) {
        return request("textDocument/hover", {
          textDocument: { uri: doc.uri.toString() }, position: pos(position),
        }).then((res) => {
          if (!res || !res.contents) return null;
          const c = res.contents;
          return new vscode.Hover(new vscode.MarkdownString(typeof c === "string" ? c : c.value));
        });
      },
    }),
    vscode.languages.registerDefinitionProvider("killer", {
      provideDefinition(doc, position) {
        return request("textDocument/definition", {
          textDocument: { uri: doc.uri.toString() }, position: pos(position),
        }).then((res) => {
          if (!res) return null;
          const locs = Array.isArray(res) ? res : [res];
          return locs.map((l) => new vscode.Location(vscode.Uri.parse(l.uri), toRange(l.range)));
        });
      },
    }),
    vscode.languages.registerDocumentSymbolProvider("killer", {
      provideDocumentSymbols(doc) {
        return request("textDocument/documentSymbol", {
          textDocument: { uri: doc.uri.toString() },
        }).then((res) => (res || []).map((s) =>
          new vscode.SymbolInformation(s.name, s.kind - 1, "",
            new vscode.Location(vscode.Uri.parse(s.location.uri), toRange(s.location.range)))));
      },
    })
  );
}

function deactivate() {
  if (!child) return undefined;
  return request("shutdown", null)
    .catch(() => {})
    .then(() => { notify("exit", null); child = null; });
}

module.exports = { activate, deactivate };
