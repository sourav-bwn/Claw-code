import * as vscode from 'vscode';
import * as path from 'path';

export class UclawChatPanel implements vscode.WebviewViewProvider {
    public static readonly viewType = 'uc law.chatView';
    private view?: vscode.WebviewView;
    private context: vscode.ExtensionContext;
    private messages: Array<{ role: string; content: string }> = [];

    constructor(context: vscode.ExtensionContext) {
        this.context = context;
    }

    public show() {
        if (this.view) {
            this.view.show?.();
        }
    }

    public ask(question: string) {
        this.messages.push({ role: 'user', content: question });
        this.updateView();
        this.getResponse(question);
    }

    resolveWebviewView(
        webviewView: vscode.WebviewView,
        context: vscode.ExtensionContext,
        token: vscode.CancellationToken
    ) {
        this.view = webviewView;
        
        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [this.context.extensionPath]
        };
        
        webviewView.webview.html = this.getHtml();
        
        webviewView.webview.onDidReceiveMessage(async (message) => {
            if (message.type === 'send') {
                this.messages.push({ role: 'user', content: message.content });
                this.updateView();
                await this.getResponse(message.content);
            } else if (message.type === 'clear') {
                this.messages = [];
                this.updateView();
            }
        });
    }

    private updateView() {
        if (!this.view) return;
        
        const messagesHtml = this.messages.map(m => `
            <div class="message ${m.role}">
                <div class="message-content">${this.escapeHtml(m.content)}</div>
            </div>
        `).join('');
        
        this.view.webview.postMessage({
            type: 'update',
            messages: messagesHtml
        });
    }

    private async getResponse(question: string): Promise<void> {
        const config = vscode.workspace.getConfiguration('uc law');
        const apiKey = config.get<string>('apiKey') || process.env.ANTHROPIC_API_KEY || process.env.OPENAI_API_KEY;
        const provider = config.get<string>('apiProvider') || 'anthropic';
        const model = config.get<string>('model') || 'claude-sonnet-4-6';
        
        if (!apiKey) {
            this.messages.push({
                role: 'assistant',
                content: '⚠️ Please set your API key in VS Code settings (uc law.apiKey) or as an environment variable (ANTHROPIC_API_KEY or OPENAI_API_KEY).'
            });
            this.updateView();
            return;
        }
        
        try {
            const messages = this.messages.map(m => ({
                role: m.role === 'user' ? 'user' : 'assistant',
                content: m.content
            }));
            
            let response: string;
            
            if (provider === 'anthropic') {
                response = await this.callAnthropic(messages, apiKey, model);
            } else if (provider === 'openai') {
                response = await this.callOpenAI(messages, apiKey, model);
            } else {
                response = 'Unsupported provider. Please use anthropic or openai.';
            }
            
            this.messages.push({ role: 'assistant', content: response });
            this.updateView();
        } catch (error: any) {
            this.messages.push({
                role: 'assistant',
                content: `❌ Error: ${error.message || 'Failed to get response'}`
            });
            this.updateView();
        }
    }

    private async callAnthropic(messages: any[], apiKey: string, model: string): Promise<string> {
        const anthropicMessages = messages.filter(m => m.role !== 'system').map(m => ({
            role: m.role,
            content: m.content
        }));
        
        const response = await fetch('https://api.anthropic.com/v1/messages', {
            method: 'POST',
            headers: {
                'x-api-key': apiKey,
                'anthropic-version': '2023-06-01',
                'content-type': 'application/json'
            },
            body: JSON.stringify({
                model,
                messages: anthropicMessages,
                max_tokens: 4096
            })
        });
        
        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }
        
        const data = await response.json();
        return data.content[0].text;
    }

    private async callOpenAI(messages: any[], apiKey: string, model: string): Promise<string> {
        const response = await fetch('https://api.openai.com/v1/chat/completions', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${apiKey}`,
                'content-type': 'application/json'
            },
            body: JSON.stringify({
                model,
                messages
            })
        });
        
        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }
        
        const data = await response.json();
        return data.choices[0].message.content;
    }

    private escapeHtml(text: string): string {
        return text
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#039;')
            .replace(/\n/g, '<br>');
    }

    private getHtml(): string {
        return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0a0a0f;
            color: #f8fafc;
            height: 100vh;
            display: flex;
            flex-direction: column;
        }
        .header {
            padding: 12px 16px;
            background: #12121a;
            border-bottom: 1px solid #1e293b;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .title {
            font-weight: 600;
            background: linear-gradient(135deg, #6366f1, #a855f7, #ec4899);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }
        .clear-btn {
            background: transparent;
            border: 1px solid #1e293b;
            color: #94a3b8;
            padding: 4px 12px;
            border-radius: 6px;
            cursor: pointer;
            font-size: 12px;
        }
        .clear-btn:hover { background: #1a1a25; }
        .messages {
            flex: 1;
            overflow-y: auto;
            padding: 16px;
            display: flex;
            flex-direction: column;
            gap: 12px;
        }
        .message { padding: 12px; border-radius: 12px; }
        .message.user {
            background: linear-gradient(135deg, #6366f1, #a855f7, #ec4899);
            color: white;
        }
        .message.assistant {
            background: #12121a;
            border: 1px solid #1e293b;
        }
        .message-content { white-space: pre-wrap; line-height: 1.6; }
        .input-area {
            padding: 12px 16px;
            background: #12121a;
            border-top: 1px solid #1e293b;
        }
        .input-container { display: flex; gap: 8px; }
        .input {
            flex: 1;
            background: #1a1a25;
            border: 1px solid #1e293b;
            border-radius: 8px;
            padding: 10px 14px;
            color: #f8fafc;
            font-size: 14px;
            resize: none;
        }
        .input:focus { outline: none; border-color: #6366f1; }
        .send-btn {
            background: linear-gradient(135deg, #6366f1, #a855f7);
            border: none;
            border-radius: 8px;
            padding: 10px 16px;
            color: white;
            font-weight: 600;
            cursor: pointer;
        }
        .send-btn:hover { opacity: 0.9; }
        .welcome {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100%;
            text-align: center;
            color: #64748b;
        }
        .welcome-icon { font-size: 48px; margin-bottom: 16px; }
        .welcome-text { font-size: 14px; }
    </style>
</head>
<body>
    <div class="header">
        <span class="title">🤖 Uclaw</span>
        <button class="clear-btn" onclick="clearChat()">Clear</button>
    </div>
    <div class="messages" id="messages">
        <div class="welcome">
            <div class="welcome-icon">🤖</div>
            <div class="welcome-text">Ask me anything about your code!</div>
        </div>
    </div>
    <div class="input-area">
        <div class="input-container">
            <textarea class="input" id="input" placeholder="Ask Uclaw..." rows="1"></textarea>
            <button class="send-btn" onclick="send()">Send</button>
        </div>
    </div>
    <script>
        const vscode = acquireVsCodeApi();
        const messagesEl = document.getElementById('messages');
        const inputEl = document.getElementById('input');
        
        function escapeHtml(text) {
            return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#039;').replace(/\\n/g, '<br>');
        }
        
        function send() {
            const text = inputEl.value.trim();
            if (!text) return;
            vscode.postMessage({ type: 'send', content: text });
            inputEl.value = '';
        }
        
        function clearChat() {
            vscode.postMessage({ type: 'clear' });
        }
        
        inputEl.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                send();
            }
        });
        
        window.addEventListener('message', (event) => {
            if (event.data.type === 'update') {
                messagesEl.innerHTML = event.data.messages;
                messagesEl.scrollTop = messagesEl.scrollHeight;
            }
        });
    </script>
</body>
</html>
        `;
    }
}
