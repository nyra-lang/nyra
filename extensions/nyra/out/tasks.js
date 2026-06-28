"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerTaskProvider = registerTaskProvider;
const vscode = __importStar(require("vscode"));
const PROBLEM_MATCHER = "$nyra";
function registerTaskProvider(context, command) {
    context.subscriptions.push(vscode.tasks.registerTaskProvider("nyra", {
        provideTasks() {
            const folder = vscode.workspace.workspaceFolders?.[0];
            if (!folder) {
                return [];
            }
            const root = folder.uri.fsPath;
            const defs = [
                { task: "build", args: ["build", "."], group: vscode.TaskGroup.Build },
                {
                    task: "build-debug",
                    label: "Nyra: build (debug)",
                    args: ["build", ".", "--debug-symbols"],
                    group: vscode.TaskGroup.Build,
                },
                { task: "run", args: ["run", "."], group: vscode.TaskGroup.Build },
                { task: "check", args: ["check", "."], group: vscode.TaskGroup.Build },
                { task: "test", args: ["test", "."], group: vscode.TaskGroup.Test },
                { task: "fmt", args: ["fmt", "--write", "."] },
            ];
            return defs.map(({ task, args, group, label }) => {
                const t = new vscode.Task({ type: "nyra", task, path: "." }, folder, label ?? `Nyra: ${task}`, "nyra", new vscode.ShellExecution(command, args, { cwd: root }), PROBLEM_MATCHER);
                if (group) {
                    t.group = group;
                }
                return t;
            });
        },
        resolveTask(task) {
            const folder = vscode.workspace.workspaceFolders?.[0];
            if (!folder || task.definition.type !== "nyra") {
                return undefined;
            }
            const name = task.definition.task;
            const taskPath = task.definition.path ?? ".";
            const argsMap = {
                build: ["build", taskPath],
                "build-debug": ["build", taskPath, "--debug-symbols"],
                run: ["run", taskPath],
                check: ["check", taskPath],
                test: ["test", taskPath],
                fmt: ["fmt", "--write", taskPath],
            };
            const args = argsMap[name] ?? ["check", taskPath];
            return new vscode.Task(task.definition, folder, task.name ?? `Nyra: ${name}`, "nyra", new vscode.ShellExecution(command, args, { cwd: folder.uri.fsPath }), PROBLEM_MATCHER);
        },
    }));
}
//# sourceMappingURL=tasks.js.map