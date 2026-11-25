import { createSocket } from "node:dgram";
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "@earendil-works/pi-ai";
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
    pi.registerTool({
        name: "skillrider_stage", label: "SkillRider stage",
        description: "Report a Skill stage when instructed by the Skill. Call running before doing the stage, then done, failed, or skipped. Use the exact heading without Markdown hashes.",
        parameters: Type.Object({
            path: Type.String(),
            heading: Type.String(),
            state: Type.Union([Type.Literal("running"), Type.Literal("done"), Type.Literal("failed"), Type.Literal("skipped")]),
        }),
        async execute(_id, args, _signal, _update, ctx) {
            send("stage", { ...args, cwd: ctx.cwd });
            return { content: [{ type: "text", text: "Stage reported." }], details: {} };
        },
    });
    pi.on("session_start", () => send("ready"));
    pi.on("session_shutdown", () => { send("session_shutdown"); });
    pi.on("agent_start", () => send("agent_start"));
    pi.on("agent_settled", () => send("agent_settled"));
    pi.on("tool_execution_start", (event, ctx) => {
        const path = typeof event.args?.path === "string" ? event.args.path : undefined;
        send(event.type, { id: event.toolCallId, name: event.toolName, cwd: ctx.cwd, path });
    });
    pi.on("tool_execution_end", event => {
        const error = event.isError ? event.result?.content?.find((part: { type: string }) => part.type === "text")?.text : undefined;
        send(event.type, { id: event.toolCallId, failed: event.isError, ms: event.durationMs,
            error: typeof error === "string" ? error.slice(0, 300) : undefined });
    });
    pi.on("message_end", ({ message }, ctx) => {
        if (message.role !== "assistant") return;
        const context = ctx.getContextUsage?.();
        send("message_end", { tokens: message.usage.totalTokens, input: message.usage.input,
            output: message.usage.output, cache: message.usage.cacheRead, cost: message.usage.cost?.total,
            context: context?.tokens, window: context?.contextWindow, model: ctx.model?.id,
            error: message.errorMessage?.slice(0, 300) });
    });
    pi.on("turn_start", () => send("activity", { label: "Turn started · waiting for model" }));
    pi.on("ui_prompt_start", () => send("activity", { label: "Waiting for user confirmation/input" }));
    pi.on("ui_prompt_end", () => send("activity", { label: "User prompt ended" }));
    pi.on("session_compact", () => send("activity", { label: "Context compacted" }));
}
