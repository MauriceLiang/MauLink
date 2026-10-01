export interface PaletteCommand { id: string; label: string; meta?: string; keywords?: string[]; disabled?: boolean; }
const normalize = (value: string) => value.normalize('NFKC').toLocaleLowerCase();
export function filterCommands(commands: PaletteCommand[], query: string) {
  const terms = normalize(query).trim().split(/\s+/).filter(Boolean);
  return commands.filter(command => terms.every(term => normalize([command.label, command.meta, ...(command.keywords ?? [])].join(' ')).includes(term)));
}
export function moveSelection(commands: PaletteCommand[], current: number, direction: 1 | -1) {
  for (let step = 1; step <= commands.length; step++) { const index = ((current + step * direction) % commands.length + commands.length) % commands.length; if (!commands[index]!.disabled) return index; }
  return -1;
}
export function isTerminalTarget(target: EventTarget | null) { return target instanceof Element && !!target.closest('.xterm'); }
