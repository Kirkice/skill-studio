import { createSocket } from "node:dgram";
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { existsSync } from "node:fs";

export default function (pi: ExtensionAPI) {
    const socket = createSocket("udp4");
    socket.unref();
    socket.on("error", () => {});
    function send(type: string, data: object = {}) {
        if (!existsSync(marker)) return;
        const packet = Buffer.from(JSON.stringify({ type, key, ...data }));
        if (packet.length <= 8192) socket.send(packet, port, "127.0.0.1", () => {});
    }
    pi.on("session_start", () => send("ready"));
    pi.on("session_shutdown", () => { send("session_shutdown"); });
    pi.on("agent_start", () => send("agent_start"));
    pi.on("agent_settled", () => send("agent_settled"));
    pi.on("tool_execution_start", (event, ctx) => {
        const path = typeof event.args?.path === "string" ? event.args.path : undefined;
        send(event.type, { id: event.toolCallId, name: event.toolName, cwd: ctx.cwd, path });
    });
    pi.on("tool_execution_end", event => send(event.type, { id: event.toolCallId, failed: event.isError, ms: event.durationMs }));
    pi.on("message_end", ({ message }) => {
        if (message.role === "assistant") send("message_end", { tokens: message.usage.totalTokens });
    });
}
