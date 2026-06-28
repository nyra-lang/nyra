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
exports.resolveNyraCommand = resolveNyraCommand;
exports.resolveDebugCommand = resolveDebugCommand;
exports.probeToolchain = probeToolchain;
exports.ensureToolchain = ensureToolchain;
exports.runNyra = runNyra;
const cp = __importStar(require("child_process"));
const fs = __importStar(require("fs"));
const os = __importStar(require("os"));
const path = __importStar(require("path"));
const vscode = __importStar(require("vscode"));
/** Resolve nyra CLI path: bundled binary (optional) or user setting. */
function resolveNyraCommand(context) {
    return resolveToolchainPath(context, "languageServerPath");
}
/** Resolve debug adapter CLI (`nyra dap`). Falls back to language server path. */
function resolveDebugCommand(context) {
    const config = vscode.workspace.getConfiguration("nyra");
    const dap = config.get("debugAdapterPath", "");
    if (dap) {
        return resolveToolchainPath(context, "debugAdapterPath");
    }
    return resolveNyraCommand(context);
}
function resolveToolchainPath(context, key) {
    const config = vscode.workspace.getConfiguration("nyra");
    const configured = config.get(key, "nyra");
    if (config.get("useBundledToolchain", false)) {
        const bundled = bundledBinaryPath(context);
        if (bundled && fs.existsSync(bundled)) {
            return bundled;
        }
    }
    return configured;
}
function bundledBinaryPath(context) {
    const override = vscode.workspace
        .getConfiguration("nyra")
        .get("bundledToolchainPath", "");
    if (override) {
        return override;
    }
    const platform = `${process.platform}-${process.arch}`;
    const candidate = path.join(context.extensionPath, "bin", `nyra-${platform}`);
    return candidate;
}
function probeToolchain(command) {
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
async function ensureToolchain(context) {
    const command = resolveNyraCommand(context);
    const info = await probeToolchain(command);
    if (!info.available) {
        const pick = await vscode.window.showWarningMessage(`Nyra CLI not found or not runnable: '${command}'. Install Nyra and ensure 'nyra lsp' works.`, "Open install guide", "Configure path");
        if (pick === "Open install guide") {
            vscode.env.openExternal(vscode.Uri.parse("https://github.com/nyra-lang/nyra#quick-start"));
        }
        else if (pick === "Configure path") {
            vscode.commands.executeCommand("workbench.action.openSettings", "nyra.languageServerPath");
        }
    }
    return info;
}
function runNyra(command, args, cwd) {
    return cp.spawn(command, args, {
        cwd,
        shell: os.platform() === "win32",
    });
}
//# sourceMappingURL=toolchain.js.map