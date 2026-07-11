import * as vscode from "vscode";
import { runNyra } from "./toolchain";

interface TestListEntry {
  file: string;
  name: string;
  line: number;
}

export function registerTestController(
  context: vscode.ExtensionContext,
  command: string
): vscode.TestController {
  const controller = vscode.tests.createTestController("nyra", "Nyra Tests");
  context.subscriptions.push(controller);

  const refresh = async (): Promise<void> => {
    controller.items.replace([]);
    const folder = vscode.workspace.workspaceFolders?.[0];
    if (!folder) {
      return;
    }
    const entries = await listTests(command, folder.uri.fsPath);
    const fileMap = new Map<string, vscode.TestItem>();
    for (const entry of entries) {
      let fileItem = fileMap.get(entry.file);
      if (!fileItem) {
        fileItem = controller.createTestItem(
          entry.file,
          pathBasename(entry.file),
          vscode.Uri.file(entry.file)
        );
        controller.items.add(fileItem);
        fileMap.set(entry.file, fileItem);
      }
      const id = `${entry.file}::${entry.name}`;
      const testItem = controller.createTestItem(
        id,
        entry.name,
        vscode.Uri.file(entry.file)
      );
      testItem.range = new vscode.Range(
        Math.max(0, entry.line - 1),
        0,
        Math.max(0, entry.line - 1),
        80
      );
      fileItem.children.add(testItem);
    }
  };

  context.subscriptions.push(
    controller.createRunProfile(
      "run",
      vscode.TestRunProfileKind.Run,
      async (request, token) => {
        await runTests(controller, command, request, token);
      },
      true
    )
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nyra.refreshTests", () => refresh())
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("nyra.runAllTests", async () => {
      const folder = vscode.workspace.workspaceFolders?.[0];
      if (!folder) {
        return;
      }
      const term = vscode.window.createTerminal("Nyra Test");
      term.show();
      term.sendText(`${command} test .`);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => {
      if (doc.languageId === "nyra") {
        void refresh();
      }
    })
  );

  void refresh();
  return controller;
}

function pathBasename(p: string): string {
  const parts = p.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] ?? p;
}

async function listTests(
  command: string,
  cwd: string
): Promise<TestListEntry[]> {
  return new Promise((resolve) => {
    const chunks: string[] = [];
    const proc = runNyra(command, ["test", ".", "--list-json"], cwd);
    proc.stdout.on("data", (d) => chunks.push(String(d)));
    proc.on("close", (code) => {
      if (code !== 0) {
        resolve([]);
        return;
      }
      try {
        resolve(JSON.parse(chunks.join("")) as TestListEntry[]);
      } catch {
        resolve([]);
      }
    });
    proc.on("error", () => resolve([]));
  });
}

async function runTests(
  controller: vscode.TestController,
  command: string,
  request: vscode.TestRunRequest,
  token: vscode.CancellationToken
): Promise<void> {
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    return;
  }
  const run = controller.createTestRun(request);
  const queue = collectTests(request);

  if (queue.length === 0) {
    run.appendOutput(`Running all tests…\n`);
    const ok = await execNyra(command, ["test", "."], folder.uri.fsPath, run);
    if (!ok) {
      run.appendOutput("Some tests failed.\n");
    }
    run.end();
    return;
  }

  for (const test of queue) {
    if (token.isCancellationRequested) {
      break;
    }
    const name = test.id.split("::").pop() ?? test.label;
    run.started(test);
    const ok = await execNyra(
      command,
      ["test", ".", "--filter", name],
      folder.uri.fsPath,
      run
    );
    if (ok) {
      run.passed(test);
    } else {
      run.failed(test, new vscode.TestMessage(`${name} failed`));
    }
  }
  run.end();
}

function collectTests(request: vscode.TestRunRequest): vscode.TestItem[] {
  const out: vscode.TestItem[] = [];
  const include = request.include;
  if (!include) {
    return out;
  }
  for (const item of include) {
    if (item.id.includes("::")) {
      out.push(item);
    } else {
      item.children.forEach((child) => out.push(child));
    }
  }
  return out;
}

function execNyra(
  command: string,
  args: string[],
  cwd: string,
  run: vscode.TestRun
): Promise<boolean> {
  return new Promise((resolve) => {
    let stdout = "";
    const proc = runNyra(command, args, cwd);
    proc.stdout.on("data", (d) => {
      const s = String(d);
      stdout += s;
      run.appendOutput(s);
    });
    proc.stderr.on("data", (d) => run.appendOutput(String(d)));
    proc.on("close", (code) => {
      // Prefer exit code; also surface explicit FAIL lines in output.
      if (/\bFAIL\b/.test(stdout) && code === 0) {
        resolve(false);
        return;
      }
      resolve(code === 0);
    });
    proc.on("error", () => resolve(false));
  });
}
