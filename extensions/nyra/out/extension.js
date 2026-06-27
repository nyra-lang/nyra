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
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const toolchain_1 = require("./toolchain");
const lspClient_1 = require("./lspClient");
const tasks_1 = require("./tasks");
const tests_1 = require("./tests");
async function activate(context) {
    const command = (0, toolchain_1.resolveNyraCommand)(context);
    const toolchain = await (0, toolchain_1.ensureToolchain)(context);
    (0, lspClient_1.startLanguageClient)(context, command, toolchain);
    (0, tasks_1.registerTaskProvider)(context, command);
    (0, tests_1.registerTestController)(context, command);
    context.subscriptions.push(vscode.debug.registerDebugAdapterDescriptorFactory("nyra", {
        createDebugAdapterDescriptor() {
            const dapPath = (0, toolchain_1.resolveDebugCommand)(context);
            return new vscode.DebugAdapterExecutable(dapPath, ["dap"]);
        },
    }));
    context.subscriptions.push(vscode.commands.registerCommand("nyra.showToolchainInfo", async () => {
        const info = await (0, toolchain_1.ensureToolchain)(context);
        const msg = info.available
            ? `Nyra ${info.version ?? ""} (${info.command})`
            : `Nyra not found (${info.command})`;
        vscode.window.showInformationMessage(msg);
    }));
}
function deactivate() {
    return (0, lspClient_1.getLanguageClient)()?.stop();
}
//# sourceMappingURL=extension.js.map