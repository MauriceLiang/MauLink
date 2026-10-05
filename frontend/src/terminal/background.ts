import type { CSSProperties } from "vue";
import type { TerminalBackgroundFit } from "../../../contracts/v1/TerminalBackgroundFit";
import type { TerminalBackgroundImageSettings } from "../../../contracts/v1/TerminalBackgroundImageSettings";
import type { TerminalBackgroundPosition } from "../../../contracts/v1/TerminalBackgroundPosition";

export function terminalBackgroundImageStyle(
  src: string,
  settings: TerminalBackgroundImageSettings,
): CSSProperties {
  return {
    backgroundImage: `url(${JSON.stringify(src)})`,
    backgroundSize: ({ cover: 'cover', contain: 'contain', stretch: '100% 100%', original: 'auto', tile: 'auto' } satisfies Record<TerminalBackgroundFit, string>)[settings.fit],
    backgroundRepeat: settings.fit === 'tile' ? 'repeat' : 'no-repeat',
    backgroundPosition: ({ center: 'center', top: 'center top', bottom: 'center bottom', left: 'left center', right: 'right center', topLeft: 'left top', topRight: 'right top', bottomLeft: 'left bottom', bottomRight: 'right bottom' } satisfies Record<TerminalBackgroundPosition, string>)[settings.position],
    opacity: settings.imageOpacity / 100,
    filter: settings.blurPx ? `blur(${settings.blurPx}px)` : undefined,
    transform: settings.blurPx ? 'scale(1.03)' : undefined,
  };
}

export function terminalBackgroundOverlayStyle(settings: TerminalBackgroundImageSettings): CSSProperties {
  return {
    backgroundColor: settings.overlayKind === 'dark' ? 'rgb(0 0 0)' : 'rgb(255 255 255)',
    opacity: settings.overlayOpacity / 100,
  };
}
