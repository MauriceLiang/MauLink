import type { ServerProfile } from "../../../../contracts/v1/ServerProfile";

function sshHost(host: string) {
  return host.includes(":") && !(host.startsWith("[") && host.endsWith("]")) ? `[${host}]` : host;
}

export function formatHostPort(host: string, port: number) {
  return `${sshHost(host)}:${port}`;
}

export function formatServerEndpoint(username: string, host: string, port: number) {
  return `${username}@${formatHostPort(host, port)}`;
}

function shellArgument(value: string) {
  return /^[A-Za-z0-9_@%+=:,./-]+$/.test(value) ? value : `'${value.replace(/'/g, "'\\''")}'`;
}

function jumpDestination(server: ServerProfile) {
  const jump = server.jumpHost;
  if (!jump) return null;
  const separator = jump.indexOf("@");
  const username = separator < 0 ? server.username : jump.slice(0, separator);
  const host = separator < 0 ? jump : jump.slice(separator + 1);
  if (!username || !host || host.includes("@")) return null;
  const destination = `${username}@${sshHost(host)}`;
  return server.jumpPort === 22 ? destination : `${destination}:${server.jumpPort}`;
}

export function buildSshCommand(server: ServerProfile): string | null {
  if (server.proxyType !== null || server.proxyHost !== null || server.proxyPort !== null) return null;
  const args = ["ssh", "-p", String(server.port)];
  if (server.jumpHost) {
    const jump = jumpDestination(server);
    if (!jump) return null;
    args.push("-J", shellArgument(jump));
  }
  args.push("--", shellArgument(`${server.username}@${sshHost(server.host)}`));
  return args.join(" ");
}

export function connectionRoute(server: ServerProfile, labels: {
  local: string;
  proxy: string;
  jumpHost: string;
  server: string;
  socks5: string;
  httpConnect: string;
}) {
  const steps = [{ label: labels.local, detail: "" }];
  if (server.proxyHost && server.proxyPort !== null && server.proxyType) {
    const type = server.proxyType === "socks5" ? labels.socks5 : labels.httpConnect;
    steps.push({ label: labels.proxy, detail: `${type} · ${formatHostPort(server.proxyHost, server.proxyPort)}` });
  }
  if (server.jumpHost) {
    const separator = server.jumpHost.indexOf("@");
    const username = separator < 0 ? server.username : server.jumpHost.slice(0, separator);
    const host = separator < 0 ? server.jumpHost : server.jumpHost.slice(separator + 1);
    steps.push({ label: labels.jumpHost, detail: formatServerEndpoint(username, host, server.jumpPort) });
  }
  steps.push({ label: labels.server, detail: formatServerEndpoint(server.username, server.host, server.port) });
  return steps;
}
