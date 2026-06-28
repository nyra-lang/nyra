import * as cp from "child_process";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";

export interface ToolchainInfo {
  command: string;
  version: string | undefined;
  available: boolean;
}

/** Resolve nyra CLI path: bundled binary (optional) or user setting. */
export function resolveNyraCommand(context: vscode.ExtensionContext): string {
  return resolveToolchainPath(context, "languageServerPath");
}

/** Resolve debug adapter CLI (`nyra dap`). Falls back to language server path. */
export function resolveDebugCommand(context: vscode.ExtensionContext): string {
  const config = vscode.workspace.getConfiguration("nyra");
  const dap = config.get<string>("debugAdapterPath", "");
  if (dap) {
    return resolveToolchainPath(context, "debugAdapterPath");
  }
  return resolveNyraCommand(context);
}

function resolveToolchainPath(
  context: vscode.ExtensionContext,
  key: "languageServerPath" | "debugAdapterPath"
): string {
  const config = vscode.workspace.getConfiguration("nyra");
  const configured = config.get<string>(key, "nyra");
  if (config.get<boolean>("useBundledToolchain", false)) {
    const bundled = bundledBinaryPath(context);
    if (bundled && fs.existsSync(bundled)) {
      return bundled;
    }
  }
  return configured;
}

function bundledBinaryPath(context: vscode.ExtensionContext): string | undefined {
  const override = vscode.workspace
    .getConfiguration("nyra")
    .get<string>("bundledToolchainPath", "");
  if (override) {
    return override;
  }
  const platform = `${process.platform}-${process.arch}`;
  const candidate = path.join(context.extensionPath, "bin", `nyra-${platform}`);
  return candidate;
}

export function probeToolchain(command: string): Promise<ToolchainInfo> {
  return new Promise((resolve) => {
    cp.execFile(command, ["--version"], { timeout: 5000 }, (err, stdout) => {
      if (err) {
        resolve({ command, version: undefined, available: false });
        return;
      }
      const version = stdout.trim().split(/\s+/)[0] ?? stdout.trim();
      resolve({ command, version, available: true });
    });
  });
}

export async function ensureToolchain(
  context: vscode.ExtensionContext
): Promise<ToolchainInfo> {
  const command = resolveNyraCommand(context);
  const info = await probeToolchain(command);
  if (!info.available) {
    const pick = await vscode.window.showWarningMessage(
      `Nyra CLI not found or not runnable: '${command}'. Install Nyra and ensure 'nyra lsp' works.`,
      "Open install guide",
      "Configure path"
    );
    if (pick === "Open install guide") {
      vscode.env.openExternal(
        vscode.Uri.parse(
          "https://github.com/nyra-lang/nyra#quick-start"
        )
      );
    } else if (pick === "Configure path") {
      vscode.commands.executeCommand(
        "workbench.action.openSettings",
        "nyra.languageServerPath"
      );
    }
  }
  return info;
}

export function runNyra(
  command: string,
  args: string[],
  cwd: string
): cp.ChildProcessWithoutNullStreams {
  return cp.spawn(command, args, {
    cwd,
    shell: os.platform() === "win32",
  });
}
