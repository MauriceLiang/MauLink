function normalize(value) {
  return String(value ?? "").normalize("NFKC").toLocaleLowerCase();
}

export function filterPaletteCommands(commands, query) {
  const terms = normalize(query).trim().split(/\s+/).filter(Boolean);
  if (!terms.length) return [...commands];
  return commands.filter((command) => {
    const searchable = normalize([
      command.label,
      command.meta,
      ...(command.keywords ?? []),
    ].filter(Boolean).join(" "));
    return terms.every((term) => searchable.includes(term));
  });
}

export function movePaletteSelection(commands, currentIndex, direction) {
  if (!commands.length || !direction) return -1;
  const start = Number.isInteger(currentIndex) ? currentIndex : -1;
  for (let step = 1; step <= commands.length; step += 1) {
    const index = ((start + direction * step) % commands.length + commands.length) % commands.length;
    if (!commands[index].disabled) return index;
  }
  return -1;
}
