import * as vscode from 'vscode';
import { UclawChatPanel } from './chatPanel';

let panel: UclawChatPanel | undefined;

export function activate(context: vscode.ExtensionContext) {
    const provider = new UclawChatPanel(context);
    
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider('uc law.chatView', provider)
    );
    
    context.subscriptions.push(
        vscode.commands.registerCommand('uc law.start', () => {
            if (!panel) {
                panel = new UclawChatPanel(context);
            }
            panel.show();
        })
    );
    
    context.subscriptions.push(
        vscode.commands.registerCommand('uc law.ask', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showInformationMessage('No active editor');
                return;
            }
            const selection = editor.document.getText(editor.selection);
            if (!selection) {
                vscode.window.showInformationMessage('No text selected');
                return;
            }
            if (!panel) {
                panel = new UclawChatPanel(context);
            }
            panel.show();
            panel.ask(`About this code:\n\`\`\`\n${selection}\n\`\`\``);
        })
    );
    
    context.subscriptions.push(
        vscode.commands.registerCommand('uc law.explain', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            const selection = editor.document.getText(editor.selection);
            if (!selection) return;
            if (!panel) {
                panel = new UclawChatPanel(context);
            }
            panel.show();
            panel.ask(`Explain this code:\n\`\`\`\n${selection}\n\`\`\``);
        })
    );
    
    context.subscriptions.push(
        vscode.commands.registerCommand('uc law.fix', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            const selection = editor.document.getText(editor.selection);
            if (!selection) return;
            if (!panel) {
                panel = new UclawChatPanel(context);
            }
            panel.show();
            panel.ask(`Fix problems in this code:\n\`\`\`\n${selection}\n\`\`\``);
        })
    );
    
    context.subscriptions.push(
        vscode.commands.registerCommand('uc law.refactor', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            const selection = editor.document.getText(editor.selection);
            if (!selection) return;
            if (!panel) {
                panel = new UclawChatPanel(context);
            }
            panel.show();
            panel.ask(`Refactor this code:\n\`\`\`\n${selection}\n\`\`\``);
        })
    );
    
    context.subscriptions.push(
        vscode.commands.registerCommand('uc law.generate', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            const selection = editor.document.getText(editor.selection);
            if (!selection) return;
            if (!panel) {
                panel = new UclawChatPanel(context);
            }
            panel.show();
            panel.ask(`Generate tests for this code:\n\`\`\`\n${selection}\n\`\`\``);
        })
    );
}

export function deactivate() {
    panel?.dispose();
}
