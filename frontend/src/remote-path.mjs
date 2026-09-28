export function parentRemotePath(path) {
  if (!path || path === "/") return "/";
  const trimmed = path.replace(/\/+$/, "");
  const separator = trimmed.lastIndexOf("/");
  return separator <= 0 ? "/" : trimmed.slice(0, separator);
}

export function joinRemotePath(parent, name) {
  const base = parent || ".";
  return base === "/" ? `/${name}` : `${base.replace(/\/+$/, "")}/${name}`;
}
