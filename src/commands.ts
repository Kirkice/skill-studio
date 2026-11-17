export type Command = { id:string; label:string; shortcut?:string; enabled:boolean; run:()=>void };
export function runShortcut(event: KeyboardEvent, commands: Command[]) {
 if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return;
 const command = commands.find(c=>c.enabled && c.shortcut === event.key.toLowerCase());
 if (command) { event.preventDefault(); command.run(); }
}
