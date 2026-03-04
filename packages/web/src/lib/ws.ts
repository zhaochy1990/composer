type WsEventHandler = (event: unknown) => void;

export class WebSocketClient {
    private ws: WebSocket | null = null;
    private handlers: WsEventHandler[] = [];
    private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    connect(url: string) {
        this.ws = new WebSocket(url);
        this.ws.onmessage = (e) => {
            try {
                const event = JSON.parse(e.data);
                this.handlers.forEach(h => h(event));
            } catch { /* ignore parse errors */ }
        };
        this.ws.onclose = () => {
            this.reconnectTimer = setTimeout(() => this.connect(url), 3000);
        };
    }

    onEvent(handler: WsEventHandler) {
        this.handlers.push(handler);
        return () => {
            this.handlers = this.handlers.filter(h => h !== handler);
        };
    }

    send(msg: unknown) {
        if (this.ws?.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(msg));
        }
    }

    disconnect() {
        if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
        this.ws?.close();
    }
}
