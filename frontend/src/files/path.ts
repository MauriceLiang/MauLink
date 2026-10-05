export function parentRemotePath(path: string) {
  if (!path || path === '/') return '/';
  const trimmed = path.replace(/\/+$/, '');
  const separator = trimmed.lastIndexOf('/');
  return separator <= 0 ? '/' : trimmed.slice(0, separator);
}
export function joinRemotePath(parent: string, name: string) {
  const base = parent || '.';
  return base === '/' ? `/${name}` : `${base.replace(/\/+$/, '')}/${name}`;
}
export function resolveMarkdownFileLink(documentPath: string, href: string): string | null {
  const value = href.trim();
  if (!value || value.startsWith('//') || /^[a-z][a-z\d+.-]*:/i.test(value)) return null;
  const encodedPath = value.split(/[?#]/, 1)[0]!;
  if (!encodedPath) return null;

  let linkedPath: string;
  try { linkedPath = decodeURIComponent(encodedPath); }
  catch { return null; }
  if (linkedPath.includes('\0')) return null;

  const segments = linkedPath.startsWith('/')
    ? []
    : parentRemotePath(documentPath).split('/').filter(Boolean);
  for (const segment of linkedPath.split('/')) {
    if (!segment || segment === '.') continue;
    if (segment === '..') segments.pop();
    else segments.push(segment);
  }
  return `/${segments.join('/')}`;
}
export function validBasename(name: string) { return !!name && name !== '.' && name !== '..' && !/[\/\0]/.test(name); }
export function breadcrumbs(path: string) {
  let current = path.startsWith('/') ? '/' : '.';
  const parts = [{ name: current, path: current }];
  for (const name of path.split('/').filter(segment => !!segment && segment !== '.')) {
    current = joinRemotePath(current, name);
    parts.push({ name, path: current });
  }
  return parts;
}
// Display calculations may be approximate; the original decimal u64 metadata stays unchanged.
export function formatSize(value: string | null) {
  if (value === null) return '—';
  let size = Number(value); if (!Number.isFinite(size) || size < 0) return '—';
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB']; let unit = 0;
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++; }
  return `${unit ? size.toFixed(1) : value} ${units[unit]}`;
}
export function progress(total: string | null, transferred: string) {
  const size = Number(total); const done = Number(transferred);
  return size > 0 && Number.isFinite(done) ? Math.min(100, Math.max(0, done / size * 100)) : null;
}
export function modifiedTime(value: number | null) {
  if (value === null) return '—';
  const date = new Date(value); if (Number.isNaN(date.valueOf())) return '—';
  return date.toLocaleString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', hourCycle: 'h23' });
}
