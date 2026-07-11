import * as vscode from "vscode";
import { ensureToolchain, resolveDebugCommand, resolveNyraCommand } from "./toolchain";
import { getLanguageClient, startLanguageClient } from "./lspClient";
import { registerTaskProvider } from "./tasks";
import { registerTestController } from "./tests";

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const command = resolveNyraCommand(context);
  const toolchain = await ensureToolchain(context);

  startLanguageClient(context, command, toolchain);
  registerTaskProvider(context, command);
  registerTestController(context, command);

  context.subscriptions.push(
    vscode.debug.registerDebugAdapterDescriptorFactory("nyra", {
      createDebugAdapterDescriptor(): vscode.DebugAdapterExecutable {
        const dapPath = resolveDebugCommand(context);
        return new vscode.DebugAdapterExecutable(dapPath, ["dap"]);
      },
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nyra.showToolchainInfo", async () => {
      const info = await ensureToolchain(context);
      const msg = info.available
        ? `Nyra ${info.version ?? ""} (${info.command})`
        : `Nyra not found (${info.command})`;
      vscode.window.showInformationMessage(msg);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand(
      "nyra.runTest",
      async (file?: string, name?: string) => {
        const folder = vscode.workspace.workspaceFolders?.[0];
        if (!folder || !name) {
          return;
        }
        const term = vscode.window.createTerminal("Nyra Test");
        term.show();
        const cwd = folder.uri.fsPath;
        const filter = name;
        term.sendText(`cd ${JSON.stringify(cwd)} && ${command} test . --filter ${JSON.stringify(filter)}`);
      }
    )
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nyra.applySourceFixAll", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== "nyra") {
        return;
      }
      const client = getLanguageClient();
      if (!client) {
        return;
      }
      await vscode.commands.executeCommand("editor.action.codeAction", {
        kind: "source.fixAll",
        apply: "first",
      });
    })
  );
}

export function deactivate(): Thenable<void> | undefined {
  return getLanguageClient()?.stop();
}
